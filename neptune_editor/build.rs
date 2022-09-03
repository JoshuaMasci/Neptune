//use shaderc::ShaderKind;
//use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let out_dir = std::env::var_os("OUT_DIR").unwrap();

    //TODO: Compiler Options
    // let mut compiler_options = shaderc::CompileOptions::new().unwrap();
    // compiler_options.set_optimization_level(shaderc::OptimizationLevel::Performance);
    // compiler_options.set_target_spirv(shaderc::SpirvVersion::V1_5);
    // compiler_options.set_source_language(shaderc::SourceLanguage::GLSL);

    // let compiler = &mut shaderc::Compiler::new().unwrap();
    //
    // let result = compile_shaders(compiler, std::path::Path::new("shader/"));
    // std::fs::write(Path::new(&out_dir).join("shader.rs"), &result).expect("Failed to write shader");
}

// fn get_shader_type(file_type: &str) -> Option<ShaderKind> {
//     match file_type {
//         "vert" => Some(ShaderKind::Vertex),
//         "frag" => Some(ShaderKind::Fragment),
//         "comp" => Some(ShaderKind::Compute),
//         &_ => None,
//     }
// }
//
// fn compile_shaders(compiler: &mut shaderc::Compiler, src_path: &std::path::Path) -> String {
//     println!("cargo:rerun-if-changed={}", src_path.to_str().unwrap());
//
//     let entries = std::fs::read_dir(src_path).expect("Failed to read shader dir");
//
//     let mut shader_code = String::new();
//
//     for entry in entries.flatten() {
//         let path = entry.path();
//         let filename = entry
//             .file_name()
//             .into_string()
//             .expect("Failed to parse filename");
//
//         println!("cargo:rerun-if-changed={}", path.to_str().unwrap());
//
//         if let Some(shader_type) = get_shader_type(path.extension().unwrap().to_str().unwrap()) {
//             let code = std::fs::read_to_string(path).expect("Failed to read shader");
//
//             let spirv_code = compiler
//                 .compile_into_spirv(&code, shader_type, &filename, "main", None)
//                 .expect("Failed to compile shader");
//
//             let output_name = filename.to_uppercase().replace('.', "_");
//             shader_code += &format!(
//                 "pub const {}: &[u32] = &{:?};",
//                 output_name,
//                 spirv_code.as_binary()
//             );
//         }
//     }
//     shader_code
// }
