//! Build script: compile build.rc into build.res via the Windows resource
//! compiler (rc.exe) so the linker embeds the application icon.
//!
//! Zero Rust-crate dependencies: it shells out to rc.exe found under the
//! Windows SDK, and leaves a .res next to OUT_DIR for the linker to pick up.
//! If rc.exe cannot be located, the build still succeeds without an icon
//! (a warning is emitted), so the project remains buildable in minimal envs.

use std::{env, path::PathBuf, process::Command};

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let rc_path = manifest_dir.join("build.rc");
    let icon_path = manifest_dir.join("icons").join("icon.ico");

    println!("cargo:rerun-if-changed=build.rc");
    println!("cargo:rerun-if-changed=icons/icon.ico");
    println!("cargo:rerun-if-changed=icons/generate-icon.ps1");

    if !rc_path.exists() || !icon_path.exists() {
        println!("cargo:warning=build.rc or icon missing; skipping icon embedding");
        return;
    }

    let rc_exe = match find_rc_exe() {
        Some(path) => path,
        None => {
            println!("cargo:warning=rc.exe not found; skipping icon embedding");
            return;
        }
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let res_path = out_dir.join("flashbridge.res");

    let rc_result = Command::new(&rc_exe)
        .arg("/nologo")
        .arg("/fo")
        .arg(&res_path)
        .arg(&rc_path)
        .output();

    match rc_result {
        Ok(output) if output.status.success() => {
            println!("cargo:rustc-link-search=native={}", out_dir.display());
            println!("cargo:rustc-link-lib=dylib:+verbatim=flashbridge.res");
        }
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            println!("cargo:warning=rc.exe failed: {stderr}");
            println!("cargo:warning=skipping icon embedding");
        }
        Err(error) => {
            println!("cargo:warning=failed to invoke rc.exe: {error}");
        }
    }
}

fn find_rc_exe() -> Option<PathBuf> {
    // 1. RC environment variable override.
    if let Ok(path) = env::var("RC") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Some(path);
        }
    }

    // 2. Where.exe / PATH lookup.
    if let Ok(output) = Command::new("where").arg("rc.exe").output() {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(first) = text.lines().next() {
                let path = PathBuf::from(first.trim());
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }

    // 3. Scan the Windows Kits installation for the matching host arch.
    let candidates = [
        r"C:\Program Files (x86)\Windows Kits\10\bin",
        r"C:\Program Files\Windows Kits\10\bin",
    ];
    let arch_filter = env::consts::ARCH; // x86_64 or aarch64
    let host_arch = match arch_filter {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "x86",
        _other => return scan_any_rc(&candidates),
    };

    for base in candidates {
        let bin_dir = match std::fs::read_dir(base) {
            Ok(entries) => entries,
            Err(_) => continue,
        };
        let mut versions: Vec<_> = bin_dir
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                entry
                    .file_name()
                    .to_str()
                    .map(String::from)
                    .filter(|name| name.starts_with("10."))
            })
            .collect();
        versions.sort();
        for version in versions.into_iter().rev() {
            let rc = PathBuf::from(base)
                .join(&version)
                .join(host_arch)
                .join("rc.exe");
            if rc.exists() {
                return Some(rc);
            }
            // Some SDKs name the dir "x86" even for host. Fallback below.
        }
    }
    scan_any_rc(&candidates)
}

fn scan_any_rc(candidates: &[&str]) -> Option<PathBuf> {
    for base in candidates {
        let walk = walk_dir(PathBuf::from(base), "rc.exe", 3);
        if let Some(path) = walk {
            return Some(path);
        }
    }
    None
}

fn walk_dir(dir: PathBuf, target: &str, depth: u32) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let entries = std::fs::read_dir(&dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = walk_dir(path, target, depth - 1) {
                return Some(found);
            }
        } else if path.file_name().and_then(|n| n.to_str()) == Some(target) {
            return Some(path);
        }
    }
    None
}
