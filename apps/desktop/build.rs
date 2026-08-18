use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    let variant = std::env::var("DSH_DESKTOP_VARIANT").unwrap_or_else(|_| "slim".to_owned());
    assert!(
        matches!(variant.as_str(), "bundled" | "slim"),
        "DSH_DESKTOP_VARIANT must be bundled or slim"
    );
    println!("cargo:rerun-if-env-changed=DSH_DESKTOP_VARIANT");
    println!("cargo:rerun-if-env-changed=DSH_DESKTOP_DEVELOPMENT");
    println!("cargo:rustc-env=DSH_DESKTOP_VARIANT={variant}");
    println!(
        "cargo:rustc-env=DSH_DESKTOP_TARGET={}",
        std::env::var("TARGET").expect("Cargo did not provide TARGET")
    );

    let manifest_dir = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo did not provide CARGO_MANIFEST_DIR"),
    );
    let development = std::env::var_os("DSH_DESKTOP_DEVELOPMENT").is_some();
    println!("cargo:rustc-env=DSH_DESKTOP_DEVELOPMENT={development}");
    generate_embedded_assets(&manifest_dir, &variant, development);
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

fn generate_embedded_assets(manifest_dir: &Path, variant: &str, development: bool) {
    let frontend = manifest_dir.join("../frontend/dist");
    let icons = manifest_dir.join("icons");
    let icon = icons.join("icon.png");
    let runtime = manifest_dir.join("runtime/bundled");

    let frontend_files = if development {
        Vec::new()
    } else {
        assert!(
            frontend.is_dir(),
            "frontend build output is missing; run `pnpm run frontend:build` first"
        );
        collect_files(&frontend)
    };
    let icon_files = vec![icon];
    let runtime_files = (variant == "bundled").then(|| {
        let files = collect_files(&runtime);
        assert!(
            !files.is_empty(),
            "bundled runtime is missing; run `pnpm run runtime:prepare` first"
        );
        files
    });

    let mut source = String::new();
    let mut resource_hash = 0xcbf2_9ce4_8422_2325_u64;
    emit_asset_group(
        &mut source,
        "FRONTEND_FILES",
        &frontend,
        &frontend_files,
        &mut resource_hash,
    );
    emit_asset_group(
        &mut source,
        "ICON_FILES",
        &icons,
        &icon_files,
        &mut resource_hash,
    );
    emit_asset_group(
        &mut source,
        "RUNTIME_SEED_FILES",
        &runtime,
        runtime_files.as_deref().unwrap_or_default(),
        &mut resource_hash,
    );
    writeln!(
        source,
        "pub const RESOURCE_ID: &str = \"{resource_hash:016x}\";"
    )
    .expect("write generated source");
    let output = PathBuf::from(std::env::var("OUT_DIR").expect("Cargo did not provide OUT_DIR"))
        .join("embedded_assets.rs");
    std::fs::write(output, source).expect("write embedded asset source");
}

fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files_at(root, &mut files);
    files.sort();
    files
}

fn collect_files_at(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("read embedded asset directory") {
        let path = entry.expect("read embedded asset entry").path();
        if path.is_dir() {
            collect_files_at(&path, files);
        } else if path.is_file() {
            println!("cargo:rerun-if-changed={}", path.display());
            files.push(path);
        }
    }
}

fn emit_asset_group(
    output: &mut String,
    name: &str,
    root: &Path,
    files: &[PathBuf],
    hash: &mut u64,
) {
    writeln!(output, "pub static {name}: &[EmbeddedFile] = &[").expect("write generated source");
    for file in files {
        let path = file
            .strip_prefix(root)
            .expect("embedded file belongs to its root")
            .to_string_lossy()
            .replace('\\', "/");
        let file_path = file.to_string_lossy();
        writeln!(
            output,
            "    EmbeddedFile {{ path: {path:?}, data: include_bytes!({file_path:?}) }},"
        )
        .expect("write generated source");
        hash_bytes(hash, path.as_bytes());
        hash_bytes(hash, &std::fs::read(file).expect("read embedded asset"));
    }
    writeln!(output, "];\n").expect("write generated source");
}

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x100_0000_01b3);
    }
}
