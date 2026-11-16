use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=assets/app.rc");
    println!("cargo:rerun-if-changed=assets/mov2mp4.ico");

    if !env::var("TARGET").unwrap_or_default().contains("windows-msvc") {
        return;
    }

    let sdk_root = PathBuf::from(env::var("ProgramFiles(x86)").expect("Windows SDK path missing"))
        .join("Windows Kits/10/bin");
    let mut versions: Vec<PathBuf> = fs::read_dir(&sdk_root)
        .expect("Windows SDK not found")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    versions.sort();
    let rc = versions
        .iter()
        .rev()
        .map(|version| version.join("x64/rc.exe"))
        .find(|path| path.is_file())
        .expect("Windows SDK resource compiler (rc.exe) not found");

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let assets = manifest.join("assets");
    let output = PathBuf::from(env::var("OUT_DIR").unwrap()).join("app.res");
    compile_resource(&rc, &assets, &output);
    println!("cargo:rustc-link-arg={}", output.display());
}

fn compile_resource(rc: &Path, assets: &Path, output: &Path) {
    let status = Command::new(rc)
        .current_dir(assets)
        .arg("/nologo")
        .arg(format!("/fo{}", output.display()))
        .arg("app.rc")
        .status()
        .expect("failed to run rc.exe");
    assert!(status.success(), "rc.exe failed to compile the icon");
}
