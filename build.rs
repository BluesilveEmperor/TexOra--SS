//! Makes sure the PDF engine (PDFium, a prebuilt library) sits next to the program.
//! It is fetched once for the current platform if it is not already in vendor/.
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn platform() -> Option<(&'static str, &'static str)> {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    match (os.as_str(), arch.as_str()) {
        ("macos", "aarch64") => Some(("mac-arm64", "libpdfium.dylib")),
        ("macos", "x86_64") => Some(("mac-x64", "libpdfium.dylib")),
        ("linux", "x86_64") => Some(("linux-x64", "libpdfium.so")),
        ("linux", "aarch64") => Some(("linux-arm64", "libpdfium.so")),
        ("windows", "x86_64") => Some(("win-x64", "pdfium.dll")),
        ("windows", "aarch64") => Some(("win-arm64", "pdfium.dll")),
        _ => None,
    }
}

fn find_lib(dir: &Path, name: &str) -> Option<PathBuf> {
    [dir.join("lib").join(name), dir.join("bin").join(name)].into_iter().find(|p| p.exists())
}

/// Puts the icon inside the Windows program, so Explorer and the taskbar show it.
fn windows_icon() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/icon.rc");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap()).join("icon.res");
    // Cross-building from a Mac uses Zig's resource compiler; on Windows the normal one is used.
    let zig = Command::new("zig").arg("rc").arg("/fo").arg(&out).arg("icon.rc").current_dir("assets").status();
    if zig.map(|s| s.success()).unwrap_or(false) {
        println!("cargo:rustc-link-arg-bins={}", out.display());
    } else if embed_resource::compile("assets/icon.rc", embed_resource::NONE).manifest_optional().is_err() {
        println!("cargo:warning=The Windows icon could not be added to the program.");
    }
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        windows_icon();
    }
    println!("cargo:rerun-if-changed=vendor/pdfium");
    let Some((plat, name)) = platform() else { return };
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let dir = root.join("vendor/pdfium").join(plat);
    if find_lib(&dir, name).is_none() {
        let _ = fs::create_dir_all(&dir);
        let url = format!("https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-{plat}.tgz");
        let tgz = dir.join("pdfium.tgz");
        let ok = Command::new("curl")
            .args(["-sSfL", "-o"])
            .arg(&tgz)
            .arg(&url)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            let _ = Command::new("tar").arg("xzf").arg(&tgz).arg("-C").arg(&dir).status();
        }
        if find_lib(&dir, name).is_none() {
            println!("cargo:warning=PDF engine could not be fetched for {plat}; put {name} next to the program.");
            return;
        }
    }
    let lib = find_lib(&dir, name).unwrap();
    // OUT_DIR is target/<profile>/build/<pkg>/<hash>/out in current cargo, and
    // target/<profile>/build/<pkg>-<hash>/out in older ones. The folder the program is built in
    // is the one above "build", whichever layout is in use; counting the levels would put the
    // library in the wrong place, and the package would then have no PDF engine.
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let profile_dir = out.ancestors().find(|p| p.file_name().is_some_and(|n| n == "build")).and_then(|p| p.parent());
    match profile_dir {
        Some(dir) => {
            if let Err(e) = fs::copy(&lib, dir.join(name)) {
                println!("cargo:warning=PDF engine could not be copied next to the program: {e}");
            }
        }
        None => println!("cargo:warning=PDF engine could not be copied next to the program: the build folder was not found."),
    }
}
