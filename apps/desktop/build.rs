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
    tauri_build::build()
}
