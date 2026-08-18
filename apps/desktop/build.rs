fn main() {
    let variant = std::env::var("DSH_DESKTOP_VARIANT").unwrap_or_else(|_| "slim".to_owned());
    assert!(
        matches!(variant.as_str(), "bundled" | "slim"),
        "DSH_DESKTOP_VARIANT must be bundled or slim"
    );
    println!("cargo:rerun-if-env-changed=DSH_DESKTOP_VARIANT");
    println!("cargo:rustc-env=DSH_DESKTOP_VARIANT={variant}");
    println!(
        "cargo:rustc-env=DSH_DESKTOP_TARGET={}",
        std::env::var("TARGET").expect("Cargo did not provide TARGET")
    );

    let manifest_dir = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo did not provide CARGO_MANIFEST_DIR"),
    );
    let webui_dir = manifest_dir.join("vendor/webui");
    println!("cargo:rerun-if-changed={}", webui_dir.display());
    let make = if cfg!(target_os = "windows") {
        "nmake"
    } else {
        "make"
    };
    let status = std::process::Command::new(make)
        .current_dir(&webui_dir)
        // Cargo's TARGET is not a compiler argument. WebUI's Makefile uses the
        // same variable internally for macOS architecture selection.
        .env_remove("TARGET")
        .status()
        .expect("failed to build the WebUI static library");
    assert!(status.success(), "failed to build the WebUI static library");
    println!(
        "cargo:rustc-link-search=native={}",
        webui_dir.join("dist").display()
    );
    println!("cargo:rustc-link-lib=static=webui-2-static");
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=pthread");
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=dl");
    }
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=user32");
        println!("cargo:rustc-link-lib=advapi32");
        println!("cargo:rustc-link-lib=shell32");
        println!("cargo:rustc-link-lib=ole32");
        println!("cargo:rustc-link-lib=ws2_32");
        println!("cargo:rustc-link-lib=uuid");
        println!("cargo:rustc-link-lib=stdc++");
    }
}
