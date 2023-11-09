use shaderc::IncludeType;
use std::path::{Path, PathBuf};

fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let output_shader_path = Path::new(&out_dir).join("shader.rs");
    compile_shaders("resource/shader/", output_shader_path);
}

/// Compiles glsl shader and embeds them into the binary
fn compile_shaders<I: AsRef<Path>, O: AsRef<Path>>(
    shader_directory_path: I,
    rust_file_output_path: O,
) {
    let compiler = shaderc::Compiler::new().expect("Failed to create shader compiler");

    let mut options =
        shaderc::CompileOptions::new().expect("Failed to create shader compiler options");
    options.set_source_language(shaderc::SourceLanguage::GLSL);
    options.set_target_env(
        shaderc::TargetEnv::Vulkan,
        shaderc::EnvVersion::Vulkan1_2 as u32,
    );
    options.set_optimization_level(shaderc::OptimizationLevel::Performance);

    {
        let shader_directory_path_ref = shader_directory_path.as_ref();
        options.set_include_callback(|include_path, include_type, source_shader, _depth| {
            let resolved_include_path = match include_type {
                IncludeType::Relative => {
                    let current_directory_path = Path::new(source_shader)
                        .parent()
                        .expect("Source shader must have a parent path");
                    current_directory_path
                }
                IncludeType::Standard => shader_directory_path_ref,
            }
            .join(include_path);

            match std::fs::read_to_string(&resolved_include_path) {
                Ok(file_content) => Ok(shaderc::ResolvedInclude {
                    resolved_name: resolved_include_path
                        .to_str()
                        .unwrap_or_default()
                        .to_string(),
                    content: file_content,
                }),
                Err(err) => Err(format!("IO Error: {}", err)),
            }
        });
    }

    let shader_file_list = list_files_recursive(shader_directory_path.as_ref());
    let mut rust_file_output = String::new();

    for shader_file in shader_file_list {
        println!(
            "cargo:rerun-if-changed={}",
            shader_file.to_str().unwrap_or_default()
        );
        if let Some(shader_kind) = get_shader_kind(
            shader_file
                .extension()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default(),
        ) {
            let file_content =
                std::fs::read_to_string(&shader_file).expect("Failed to read shader file");

            let compilation_artifact = compiler
                .compile_into_spirv(
                    &file_content,
                    shader_kind,
                    shader_file
                        .to_str()
                        .expect("Failed to convert path to string"),
                    "main", //TODO: different entry points? not sure why this is needed
                    Some(&options),
                )
                .unwrap_or_else(|err| {
                    panic!("Failed to compile shader {:?}: {}", shader_file, err)
                });

            let shader_binary = compilation_artifact.as_binary();

            let shader_name = shader_file
                .strip_prefix(shader_directory_path.as_ref())
                .expect("Failed to strip directory")
                .to_str()
                .expect("Failed to convert path to string")
                .replace(['.', '/'], "_")
                .to_uppercase();

            rust_file_output += &format!(
                "#[allow(unused)]\n pub const {}: &[u32] = &{:?};\n",
                shader_name, shader_binary,
            );
        }
    }

    std::fs::write(rust_file_output_path, rust_file_output)
        .expect("Failed to output compiled shader rust file");
}

/// Returns None if the extension is not a valid shader kind
fn get_shader_kind(extension: &str) -> Option<shaderc::ShaderKind> {
    match extension {
        "vert" => Some(shaderc::ShaderKind::Vertex),
        "frag" => Some(shaderc::ShaderKind::Fragment),
        "comp" => Some(shaderc::ShaderKind::Compute),
        _ => None,
    }
}

fn list_files_recursive<P: AsRef<Path>>(dir: P) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if dir.as_ref().is_dir() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    files.push(path);
                } else if path.is_dir() {
                    let mut sub_files = list_files_recursive(&path);
                    files.append(&mut sub_files);
                }
            }
        }
    } else {
        panic!("Input path is not a directory");
    }

    files
}
