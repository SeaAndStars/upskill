//! 编译 WGSL 着色器；Windows 下可选链接 Vulkan SDK。

use std::fs;
use std::path::Path;

fn compile_wgsl(path: &str, stage: naga::ShaderStage, entry: &str) -> Vec<u8> {
    let source = fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let module = naga::front::wgsl::parse_str(&source).unwrap_or_else(|e| panic!("parse {path}: {e}"));
    let info = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    )
    .validate(&module)
    .unwrap_or_else(|e| panic!("validate {path}: {e}"));
    let options = naga::back::spv::Options::default();
    let pipeline = naga::back::spv::PipelineOptions {
        shader_stage: stage,
        entry_point: entry.into(),
    };
    let spv = naga::back::spv::write_vec(&module, &info, &options, Some(&pipeline))
        .unwrap_or_else(|e| panic!("spv {path}: {e}"));
    let mut bytes = Vec::new();
    for w in spv {
        bytes.extend_from_slice(&w.to_le_bytes());
    }
    bytes
}

fn link_vulkan_sdk() {
    if let Ok(sdk) = std::env::var("VULKAN_SDK") {
        let lib_dir = Path::new(&sdk).join("Lib");
        if lib_dir.exists() {
            println!("cargo:rustc-link-search={}", lib_dir.display());
            println!("cargo:rustc-link-lib=vulkan-1");
        }
    }
}

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let write = |name: &str, bytes: &[u8]| {
        fs::write(Path::new(&out_dir).join(name), bytes).expect("write spv");
    };
    write(
        "color.vert.spv",
        &compile_wgsl("shaders/vert.wgsl", naga::ShaderStage::Vertex, "vs_main"),
    );
    write(
        "line.frag.spv",
        &compile_wgsl("shaders/line.frag.wgsl", naga::ShaderStage::Fragment, "main"),
    );
    write(
        "point.frag.spv",
        &compile_wgsl("shaders/point.frag.wgsl", naga::ShaderStage::Fragment, "main"),
    );
    println!("cargo:rerun-if-changed=shaders/");
    link_vulkan_sdk();
}
