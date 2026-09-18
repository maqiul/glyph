fn main() {
    tauri_build::build();

    // macOS：编译原生 Vision OCR 的 Objective-C shim，并链接所需系统框架。
    // 用编译期 cfg（build script 以 host 平台编译；CI 在本机原生构建，host==target）。
    #[cfg(target_os = "macos")]
    {
        cc::Build::new()
            .file("src/ocr_vision.m")
            .flag("-fobjc-arc")
            .flag("-mmacosx-version-min=11.0")
            .flag_if_supported("-Wno-deprecated-declarations")
            .compile("ocr_vision");
        for framework in ["Vision", "CoreGraphics", "ImageIO", "Foundation", "CoreFoundation"] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
        println!("cargo:rerun-if-changed=src/ocr_vision.m");
    }
}
