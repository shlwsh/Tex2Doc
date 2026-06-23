fn main() {
    let version = std::fs::read_to_string("VERSION")
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|_| "1.26.6.1".to_string());
    println!("cargo:rerun-if-changed=VERSION");
    println!("cargo:rerun-if-changed=src/ui/assets/app_icon.jpg");
    println!("cargo:rustc-env=TEX2DOC_DESKTOP_VERSION={version}");
    slint_build::compile("src/ui/main.slint").unwrap();
}
