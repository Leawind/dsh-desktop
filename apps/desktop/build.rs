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
    let windows_icon = generate_windows_icon(&manifest_dir);
    compile_windows_icon(&windows_icon);
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
    }
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=framework=Cocoa");
        println!("cargo:rustc-link-lib=framework=WebKit");
    }
}

fn generate_embedded_assets(manifest_dir: &Path, variant: &str, development: bool) {
    let frontend = manifest_dir.join("../frontend/dist");
    let icon_root = manifest_dir.join("../frontend/public");
    let icon = application_icon(manifest_dir);
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
        &icon_root,
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

fn application_icon(manifest_dir: &Path) -> PathBuf {
    manifest_dir.join("../frontend/public/app-icon.png")
}

fn generate_windows_icon(manifest_dir: &Path) -> PathBuf {
    let source = application_icon(manifest_dir);
    println!("cargo:rerun-if-changed={}", source.display());
    let file = std::fs::File::open(&source).expect("read application icon");
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().expect("read application icon metadata");
    assert_eq!(reader.info().color_type, png::ColorType::Rgba);
    assert_eq!(reader.info().bit_depth, png::BitDepth::Eight);
    assert_eq!(reader.info().width, reader.info().height);
    let mut pixels = vec![0; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut pixels)
        .expect("decode application icon");
    pixels.truncate(info.buffer_size());
    assert_eq!(pixels.len(), (info.width * info.height * 4) as usize);

    let icon = PathBuf::from(std::env::var("OUT_DIR").expect("Cargo did not provide OUT_DIR"))
        .join("dsh-desktop.ico");
    let mut images = Vec::new();
    for size in [16, 24, 32, 48, 64, 256] {
        images.push((
            size,
            encode_icon_bitmap(size, info.width, info.height, &pixels),
        ));
    }
    write_icon_directory(&icon, &images);
    icon
}

fn encode_icon_bitmap(size: u32, source_width: u32, source_height: u32, source: &[u8]) -> Vec<u8> {
    let mask_row_bytes = size.div_ceil(32) * 4;
    let mut bitmap = Vec::with_capacity((40 + size * size * 4 + size * mask_row_bytes) as usize);
    bitmap.extend_from_slice(&40_u32.to_le_bytes());
    bitmap.extend_from_slice(&(size as i32).to_le_bytes());
    bitmap.extend_from_slice(&((size * 2) as i32).to_le_bytes());
    bitmap.extend_from_slice(&1_u16.to_le_bytes());
    bitmap.extend_from_slice(&32_u16.to_le_bytes());
    bitmap.extend_from_slice(&0_u32.to_le_bytes());
    bitmap.extend_from_slice(&0_u32.to_le_bytes());
    bitmap.extend_from_slice(&0_i32.to_le_bytes());
    bitmap.extend_from_slice(&0_i32.to_le_bytes());
    bitmap.extend_from_slice(&0_u32.to_le_bytes());
    bitmap.extend_from_slice(&0_u32.to_le_bytes());

    for y in (0..size).rev() {
        let source_y = ((y * source_height) / size).min(source_height - 1);
        for x in 0..size {
            let source_x = ((x * source_width) / size).min(source_width - 1);
            let offset = ((source_y * source_width + source_x) * 4) as usize;
            bitmap.extend_from_slice(&[
                source[offset + 2],
                source[offset + 1],
                source[offset],
                source[offset + 3],
            ]);
        }
    }
    bitmap.resize(bitmap.len() + (size * mask_row_bytes) as usize, 0);
    bitmap
}

fn write_icon_directory(icon: &Path, images: &[(u32, Vec<u8>)]) {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, image) in images {
        bytes.push(if *size >= 256 { 0 } else { *size as u8 });
        bytes.push(if *size >= 256 { 0 } else { *size as u8 });
        bytes.push(0);
        bytes.push(0);
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&32_u16.to_le_bytes());
        bytes.extend_from_slice(&(image.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&offset.to_le_bytes());
        offset += image.len() as u32;
    }
    for (_, image) in images {
        bytes.extend_from_slice(image);
    }
    std::fs::write(icon, bytes).expect("write Windows application icon");
}

fn compile_windows_icon(icon: &Path) {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let output_directory = icon.parent().expect("Windows icon has a parent directory");
    let resource = output_directory.join("dsh-desktop.res");
    let definition = output_directory.join("dsh-desktop.rc");
    let icon_path = icon.to_string_lossy().replace('\\', "/");
    std::fs::write(&definition, format!("1 ICON \"{icon_path}\"\n"))
        .expect("write Windows icon resource definition");
    let output = std::process::Command::new("rc.exe")
        .arg("/nologo")
        .arg(format!("/fo{}", resource.display()))
        .arg(&definition)
        .output()
        .expect("run Windows resource compiler");
    assert!(
        output.status.success(),
        "compile Windows application icon: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    println!(
        "cargo:rustc-link-arg-bin=dsh-desktop={}",
        resource.display()
    );
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
