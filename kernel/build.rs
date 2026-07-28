// build.rs: Compile CUDA kernels and link them
//
// Requires:
// - CUDA Toolkit installed (nvcc compiler)
// - CUDA_PATH environment variable set, or nvcc on PATH
//
// Usage:
//   cargo build --features cuda  (to include CUDA)
//   cargo build                   (CPU-only fallback)

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let cuda_feature = cfg!(feature = "cuda");

    if cuda_feature {
        compile_cuda_kernels();
    } else {
        println!("cargo:warning=CUDA feature not enabled; building CPU-only kernel");
    }
}

fn compile_cuda_kernels() {
    // 1. Find CUDA installation
    let cuda_path = find_cuda_path();
    println!("cargo:warning=Using CUDA at: {:?}", cuda_path);

    // 2. Detect GPU architecture (default: sm_75 for compatibility, supports T4 and A100)
    let gpu_arch = env::var("GPU_ARCH").unwrap_or_else(|_| "sm_75".to_string());
    println!("cargo:warning=Compiling for GPU architecture: {}", gpu_arch);

    // 3. Output directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let cuda_lib_dir = out_dir.join("cuda_kernels");
    std::fs::create_dir_all(&cuda_lib_dir).unwrap();

    // 4. Compile CUDA kernels
    compile_cuda_file(
        "src/cuda/brain_delta.cu",
        &cuda_path,
        &gpu_arch,
        &cuda_lib_dir,
        "brain_delta",
    );

    compile_cuda_file(
        "src/cuda/brain_gamma.cu",
        &cuda_path,
        &gpu_arch,
        &cuda_lib_dir,
        "brain_gamma",
    );

    // 5. Link CUDA libraries
    link_cuda(&cuda_path, &out_dir);

    // 6. Output library paths
    println!("cargo:rustc-link-search=native={}", cuda_lib_dir.display());
    println!("cargo:rustc-link-search=native={}/lib64", cuda_path.display());
}

fn find_cuda_path() -> PathBuf {
    // Try environment variable first
    if let Ok(cuda_path) = env::var("CUDA_PATH") {
        return PathBuf::from(cuda_path);
    }

    // Try common Linux paths
    let common_paths = vec![
        "/usr/local/cuda",
        "/usr/local/cuda-12.4",
        "/usr/local/cuda-12.3",
        "/opt/cuda",
        "/opt/nvidia/cuda",
    ];

    for path in common_paths {
        let cuda_path = PathBuf::from(path);
        if cuda_path.join("bin/nvcc").exists() || cuda_path.join("bin/nvcc.exe").exists() {
            return cuda_path;
        }
    }

    // Fall back to system PATH (assume nvcc is available)
    PathBuf::from("/usr/local/cuda")
}

fn compile_cuda_file(
    cu_file: &str,
    cuda_path: &std::path::Path,
    gpu_arch: &str,
    output_dir: &std::path::Path,
    kernel_name: &str,
) {
    let nvcc = cuda_path.join("bin/nvcc");
    let output_file = output_dir.join(format!("lib{}.a", kernel_name));

    println!("cargo:warning=Compiling {} with nvcc...", cu_file);

    let output = Command::new(&nvcc)
        .arg("-arch").arg(gpu_arch)
        .arg("-O3")
        .arg("-std=c++17")
        .arg("-c")
        .arg(cu_file)
        .arg("-o").arg(&output_file)
        .output();

    match output {
        Ok(result) => {
            if !result.status.success() {
                let stderr = String::from_utf8_lossy(&result.stderr);
                println!("cargo:warning=CUDA compilation error:\n{}", stderr);
                panic!("Failed to compile CUDA kernel: {}", cu_file);
            }
        }
        Err(e) => {
            println!("cargo:warning=Failed to run nvcc: {}", e);
            println!("cargo:warning=Make sure CUDA Toolkit is installed and nvcc is on PATH");
            panic!("Could not find nvcc compiler");
        }
    }

    // Link the compiled object file
    println!("cargo:rustc-link-lib=static={}", kernel_name);
}

fn link_cuda(cuda_path: &std::path::Path, _out_dir: &std::path::Path) {
    // Link CUDA runtime
    let cuda_lib_dir = cuda_path.join("lib64");
    println!("cargo:rustc-link-search=native={}", cuda_lib_dir.display());
    println!("cargo:rustc-link-lib=cudart");

    // On Linux, may need to specify rpath
    let cuda_lib_path = cuda_lib_dir.display();
    println!("cargo:rustc-link-search=native={}", cuda_lib_path);

    // Set rpath for runtime linking
    #[cfg(target_os = "linux")]
    {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}:$ORIGIN/../lib", cuda_lib_path);
    }
}
