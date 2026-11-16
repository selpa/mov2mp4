use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const FFMPEG: &[u8] = include_bytes!("../vendor/ffmpeg.zlib");
const FFMPEG_LICENSE: &str = include_str!("../vendor/FFMPEG_LICENSE.txt");

struct EmbeddedFfmpeg {
    directory: PathBuf,
    executable: PathBuf,
}

impl EmbeddedFfmpeg {
    fn new() -> Result<Self, String> {
        let directory = env::temp_dir().join(format!(
            "mov2mp4-{}-{}",
            process::id(),
            timestamp()?
        ));
        let decompressed = miniz_oxide::inflate::decompress_to_vec_zlib(FFMPEG)
            .map_err(|error| format!("내장 FFmpeg 압축을 풀 수 없습니다: {error:?}"))?;
        fs::create_dir(&directory)
            .map_err(|error| format!("FFmpeg 임시 폴더를 만들 수 없습니다: {error}"))?;
        let executable = directory.join("ffmpeg.exe");
        if let Err(error) = fs::write(&executable, decompressed) {
            let _ = fs::remove_dir_all(&directory);
            return Err(format!("내장 FFmpeg을 임시 폴더에 저장할 수 없습니다: {error}"));
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
    if let Err(message) = run() {
        eprintln!("오류: {message}");
        process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let input = match args.next() {
        Some(arg) if arg == "--help" || arg == "-h" => {
            print_usage();
            return Ok(());
        }
        Some(arg) if arg == "--license" => {
            println!("이 프로그램에는 FFmpeg(https://ffmpeg.org)가 포함되어 있습니다.\n");
            print!("{FFMPEG_LICENSE}");
            return Ok(());
        }
        Some(arg) => PathBuf::from(arg),
        None => {
            print_usage();
            return Err("MOV 파일 경로를 지정하세요.".into());
        }
    };
    if args.next().is_some() {
        return Err("한 번에 하나의 MOV 파일만 지정할 수 있습니다.".into());
    }
    if !input.is_file() {
        return Err(format!("입력 파일을 찾을 수 없습니다: {}", input.display()));
    }
    if !input
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("mov"))
    {
        return Err("확장자가 .mov인 파일을 지정하세요.".into());
    }

    let output = input.with_extension("mp4");
    if output.exists() {
        return Err(format!("출력 파일이 이미 있습니다: {}", output.display()));
    }

    let started = Instant::now();
    let temporary = temporary_path(&output)?;
    let ffmpeg = EmbeddedFfmpeg::new()?;

    let result = Command::new(ffmpeg.path())
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
                return Err(format!("변환 중 출력 파일이 생성되었습니다: {}", output.display()));
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

fn temporary_path(output: &Path) -> Result<PathBuf, String> {
    let now = timestamp()?;
    let stem = output
        .file_stem()
        .ok_or("출력 파일 이름을 만들 수 없습니다.")?;
    let mut name = stem.to_os_string();
    name.push(format!(".{}.{}.tmp.mp4", process::id(), now));
    Ok(output.with_file_name(name))
}

fn timestamp() -> Result<u128, String> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("임시 파일 이름을 만들 수 없습니다: {error}"))?
        .as_nanos())
}

fn print_usage() {
    println!("사용법: mov2mp4.exe <파일.mov>");
    println!("입력 파일과 같은 폴더에 .mp4 파일을 만듭니다. 원본은 유지합니다.");
    println!("내장 FFmpeg의 라이선스: mov2mp4.exe --license");
}
