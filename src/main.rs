use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const FFMPEG: &[u8] = include_bytes!("../vendor/ffmpeg.zlib");
const FFMPEG_LICENSE: &str = include_str!("../vendor/FFMPEG_LICENSE.txt");
const LIBHEIF_LICENSE: &str = include_str!("../vendor/LIBHEIF_LICENSE.txt");
const LIBHEIF_ARCHIVE: &[u8] = include_bytes!("../vendor/libheif-1.23.4-win64.7z");
const SEVEN_ZR: &[u8] = include_bytes!("../vendor/7zr.exe");

#[derive(Clone, Copy)]
enum Kind {
    Mov,
    Heif,
}

impl Kind {
    fn matches(self, path: &Path) -> bool {
        path.extension().is_some_and(|extension| match self {
            Self::Mov => extension.eq_ignore_ascii_case("mov"),
            Self::Heif => {
                extension.eq_ignore_ascii_case("heic") || extension.eq_ignore_ascii_case("heif")
            }
        })
    }
}

struct EmbeddedHeif {
    directory: PathBuf,
    executable: PathBuf,
}

impl EmbeddedHeif {
    fn new() -> Result<Self, String> {
        let directory =
            env::temp_dir().join(format!("mov2mp4-heif-{}-{}", process::id(), timestamp()?));
        fs::create_dir(&directory)
            .map_err(|error| format!("HEIF 임시 폴더를 만들 수 없습니다: {error}"))?;
        let archive = directory.join("libheif.7z");
        let extractor = directory.join("7zr.exe");
        let result = (|| {
            fs::write(&archive, LIBHEIF_ARCHIVE)
                .map_err(|error| format!("내장 HEIF 라이브러리를 저장할 수 없습니다: {error}"))?;
            fs::write(&extractor, SEVEN_ZR)
                .map_err(|error| format!("내장 압축 해제기를 저장할 수 없습니다: {error}"))?;
            let output = Command::new(&extractor)
                .arg("x")
                .arg(&archive)
                .arg(format!("-o{}", directory.display()))
                .arg("-y")
                .output()
                .map_err(|error| format!("HEIF 라이브러리 압축을 풀 수 없습니다: {error}"))?;
            if !output.status.success() {
                return Err(format!(
                    "HEIF 라이브러리 압축 해제 실패: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            Ok(directory.join("libheif-1.23.4-win64").join("heif-dec.exe"))
        })();
        let _ = fs::remove_file(&extractor);
        let _ = fs::remove_file(&archive);
        match result {
            Ok(executable) if executable.is_file() => Ok(Self {
                directory,
                executable,
            }),
            Ok(_) => {
                let _ = fs::remove_dir_all(&directory);
                Err("내장 HEIF 변환기를 찾을 수 없습니다.".into())
            }
            Err(error) => {
                let _ = fs::remove_dir_all(&directory);
                Err(error)
            }
        }
    }

    fn path(&self) -> &Path {
        &self.executable
    }
}

impl Drop for EmbeddedHeif {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.directory);
    }
}

struct EmbeddedFfmpeg {
    directory: PathBuf,
    executable: PathBuf,
}

impl EmbeddedFfmpeg {
    fn new() -> Result<Self, String> {
        let directory = env::temp_dir().join(format!("mov2mp4-{}-{}", process::id(), timestamp()?));
        let decompressed = miniz_oxide::inflate::decompress_to_vec_zlib(FFMPEG)
            .map_err(|error| format!("내장 FFmpeg 압축을 풀 수 없습니다: {error:?}"))?;
        fs::create_dir(&directory)
            .map_err(|error| format!("FFmpeg 임시 폴더를 만들 수 없습니다: {error}"))?;
        let executable = directory.join("ffmpeg.exe");
        if let Err(error) = fs::write(&executable, decompressed) {
            let _ = fs::remove_dir_all(&directory);
            return Err(format!(
                "내장 FFmpeg을 임시 폴더에 저장할 수 없습니다: {error}"
            ));
        }
        Ok(Self {
            directory,
            executable,
        })
    }

    fn path(&self) -> &Path {
        &self.executable
    }
}

impl Drop for EmbeddedFfmpeg {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.executable);
        let _ = fs::remove_dir(&self.directory);
    }
}

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => process::exit(1),
        Err(message) => {
            eprintln!("오류: {message}");
            process::exit(1);
        }
    }
}

fn run() -> Result<bool, String> {
    let mut args = env::args_os().skip(1);
    let input = match args.next() {
        Some(arg) if arg == "--help" || arg == "-h" => {
            print_usage();
            return Ok(true);
        }
        Some(arg) if arg == "--license" => {
            println!("이 프로그램에는 FFmpeg(https://ffmpeg.org)가 포함되어 있습니다.\n");
            print!("{FFMPEG_LICENSE}");
            println!("\n이 프로그램에는 libheif(https://github.com/strukturag/libheif)가 포함되어 있습니다.\n");
            print!("{LIBHEIF_LICENSE}");
            println!("\n압축 해제에는 7-Zip의 7zr.exe(https://www.7-zip.org)를 사용합니다.\n");
            return Ok(true);
        }
        Some(arg) => PathBuf::from(arg),
        None => {
            print_usage();
            return Err("MOV, HEIC, HEIF 파일 또는 해당 *.확장자 패턴을 지정하세요.".into());
        }
    };
    if args.next().is_some() {
        return Err("파일 경로 또는 *.확장자 패턴 하나를 지정하세요.".into());
    }

    let (kind, inputs) = collect_inputs(&input)?;
    let ffmpeg = if matches!(kind, Kind::Mov) {
        Some(EmbeddedFfmpeg::new()?)
    } else {
        None
    };
    let heif = if matches!(kind, Kind::Heif) {
        Some(EmbeddedHeif::new()?)
    } else {
        None
    };
    let mut all_succeeded = true;
    for file in inputs {
        let result = match kind {
            Kind::Mov => convert_mov(&file, ffmpeg.as_ref().unwrap().path()),
            Kind::Heif => convert_heif(&file, heif.as_ref().unwrap().path()),
        };
        if let Err(message) = result {
            eprintln!("오류: {}: {message}", file.display());
            all_succeeded = false;
        }
    }
    Ok(all_succeeded)
}

fn collect_inputs(input: &Path) -> Result<(Kind, Vec<PathBuf>), String> {
    let wildcard = input
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| {
            if name.eq_ignore_ascii_case("*.mov") {
                Some((Kind::Mov, "mov"))
            } else if name.eq_ignore_ascii_case("*.heic") {
                Some((Kind::Heif, "heic"))
            } else if name.eq_ignore_ascii_case("*.heif") {
                Some((Kind::Heif, "heif"))
            } else {
                None
            }
        });
    if let Some((kind, extension)) = wildcard {
        let directory = input
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let entries = fs::read_dir(directory).map_err(|error| {
            format!("폴더를 읽을 수 없습니다 ({}): {error}", directory.display())
        })?;
        let mut files = Vec::new();
        for entry in entries {
            let path = entry
                .map_err(|error| format!("폴더 항목을 읽을 수 없습니다: {error}"))?
                .path();
            if path.is_file()
                && path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
            {
                files.push(path);
            }
        }
        files.sort();
        if files.is_empty() {
            return Err(format!("해당 파일이 없습니다: {}", input.display()));
        }
        Ok((kind, files))
    } else if !input.is_file() {
        Err(format!("입력 파일을 찾을 수 없습니다: {}", input.display()))
    } else {
        let kind = if Kind::Mov.matches(input) {
            Kind::Mov
        } else if Kind::Heif.matches(input) {
            Kind::Heif
        } else {
            return Err("확장자가 .mov, .heic, .heif인 파일을 지정하세요.".into());
        };
        Ok((kind, vec![input.to_path_buf()]))
    }
}

fn convert_mov(input: &Path, ffmpeg: &Path) -> Result<(), String> {
    let output = input.with_extension("mp4");
    if output.exists() {
        return Err(format!("출력 파일이 이미 있습니다: {}", output.display()));
    }

    let started = Instant::now();
    let temporary = temporary_path(&output)?;

    let result = Command::new(ffmpeg)
        .arg("-nostdin")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-n")
        .arg("-i")
        .arg(&input)
        .arg("-map")
        .arg("0:v:0")
        .arg("-map")
        .arg("0:a?")
        .arg("-c")
        .arg("copy")
        .arg("-movflags")
        .arg("+faststart")
        .arg(&temporary)
        .status();

    match result {
        Ok(status) if status.success() => {
            if output.exists() {
                let _ = fs::remove_file(&temporary);
                return Err(format!(
                    "변환 중 출력 파일이 생성되었습니다: {}",
                    output.display()
                ));
            }
            fs::rename(&temporary, &output).map_err(|error| {
                let _ = fs::remove_file(&temporary);
                format!("출력 파일을 저장할 수 없습니다: {error}")
            })?;
            println!(
                "변환 : {} → {} [{:.2} 초]",
                input.display(),
                output.display(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Ok(status) => {
            let _ = fs::remove_file(&temporary);
            Err(format!("FFmpeg 변환 실패 (종료 코드: {status})"))
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(format!("내장 FFmpeg을 실행할 수 없습니다: {error}"))
        }
    }
}

fn convert_heif(input: &Path, heif_decoder: &Path) -> Result<(), String> {
    let output = input.with_extension("jpg");
    if output.exists() {
        return Err(format!("출력 파일이 이미 있습니다: {}", output.display()));
    }

    let started = Instant::now();
    let temporary = temporary_path(&output)?;
    let result = Command::new(heif_decoder)
        .arg("--quiet")
        .arg("--quality")
        .arg("92")
        .arg(input)
        .arg(&temporary)
        .output();

    match result {
        Ok(result) if result.status.success() && temporary.is_file() => {
            if output.exists() {
                let _ = fs::remove_file(&temporary);
                return Err(format!(
                    "변환 중 출력 파일이 생성되었습니다: {}",
                    output.display()
                ));
            }
            fs::rename(&temporary, &output).map_err(|error| {
                let _ = fs::remove_file(&temporary);
                format!("출력 파일을 저장할 수 없습니다: {error}")
            })?;
            println!(
                "변환 : {} → {} [{:.2} 초]",
                input.display(),
                output.display(),
                started.elapsed().as_secs_f64()
            );
            Ok(())
        }
        Ok(result) => {
            let _ = fs::remove_file(&temporary);
            let detail = String::from_utf8_lossy(&result.stderr);
            Err(format!(
                "HEIC/HEIF 변환 실패 ({}): {}",
                result.status,
                detail.trim()
            ))
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            Err(format!("내장 HEIF 변환기를 실행할 수 없습니다: {error}"))
        }
    }
}

fn temporary_path(output: &Path) -> Result<PathBuf, String> {
    let now = timestamp()?;
    let stem = output
        .file_stem()
        .ok_or("출력 파일 이름을 만들 수 없습니다.")?;
    let mut name = stem.to_os_string();
    let extension = output
        .extension()
        .ok_or("출력 파일 확장자가 없습니다.")?
        .to_string_lossy();
    name.push(format!(".{}.{}.tmp.{extension}", process::id(), now));
    Ok(output.with_file_name(name))
}

fn timestamp() -> Result<u128, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("임시 파일 이름을 만들 수 없습니다: {error}"))?
        .as_nanos())
}

fn print_usage() {
    println!("사용법: mov2mp4.exe <파일.mov | 파일.heic | 파일.heif | *.mov | *.heic | *.heif | 폴더\\*.확장자>");
    println!("MOV는 MP4로, HEIC/HEIF는 JPG로 같은 폴더에 변환합니다. 원본은 유지합니다.");
    println!("내장 도구의 라이선스: mov2mp4.exe --license");
}
