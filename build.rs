use std::{path::PathBuf, process::Command};

fn main() {
    println!("cargo:rerun-if-changed=assets/InkPage.ico");
    println!("cargo:rerun-if-env-changed=RC");
    println!("cargo:rerun-if-env-changed=WindowsSdkDir");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    assert_eq!(
        std::env::var("CARGO_CFG_TARGET_ENV").as_deref(),
        Ok("msvc"),
        "Windows icon resources require the documented MSVC toolchain"
    );
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let output = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let numeric = format!("{},0", version.replace('.', ","));
    let icon = root
        .join("assets/InkPage.ico")
        .display()
        .to_string()
        .replace('\\', "\\\\");
    let resource = output.join("inkpage.rc");
    let compiled = output.join("inkpage.res");
    std::fs::write(
        &resource,
        format!(
            r#"
1 ICON "{icon}"
1 VERSIONINFO
FILEVERSION {numeric}
PRODUCTVERSION {numeric}
FILEOS 0x40004
FILETYPE 0x1
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "InkPage Text Editor\0"
      VALUE "FileVersion", "{version}\0"
      VALUE "ProductName", "InkPage\0"
      VALUE "ProductVersion", "{version}\0"
      VALUE "OriginalFilename", "InkPage.exe\0"
      VALUE "LegalCopyright", "MIT License\0"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x0409, 1200
  END
END
"#
        ),
    )
    .expect("write Windows resource source");
    let compiler = find_rc().expect("Windows SDK rc.exe not found; install the Windows SDK with MSVC Build Tools, or set RC to its executable path");
    let status = Command::new(compiler)
        .arg("/nologo")
        .arg("/fo")
        .arg(&compiled)
        .arg(&resource)
        .status()
        .expect("run Windows resource compiler");
    assert!(status.success(), "Windows icon resource compilation failed");
    println!("cargo:rustc-link-arg-bin=rustpad={}", compiled.display());
}
fn find_rc() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("RC") {
        return Some(path.into());
    }
    if let Some(paths) = std::env::var_os("PATH") {
        for path in std::env::split_paths(&paths) {
            let compiler = path.join("rc.exe");
            if compiler.is_file() {
                return Some(compiler);
            }
        }
    }
    let sdk = std::env::var_os("WindowsSdkDir")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("ProgramFiles(x86)")
                .map(|path| PathBuf::from(path).join("Windows Kits/10"))
        })?;
    let mut versions: Vec<_> = std::fs::read_dir(sdk.join("bin"))
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    versions.sort();
    versions
        .into_iter()
        .rev()
        .map(|path| path.join("x64/rc.exe"))
        .find(|path| path.is_file())
}
