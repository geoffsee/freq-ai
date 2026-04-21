use std::path::Path;
use std::process::Command;

fn main() {
    let wasm_path = Path::new("dist/wasm/freq-ai_bg.wasm");

    println!("cargo::rerun-if-changed={}", wasm_path.display());

    if !wasm_path.exists() {
        return;
    }

    let meta = std::fs::metadata(wasm_path).expect("failed to read wasm metadata");
    // Only optimize if larger than 10MB (i.e. not already optimized)
    if meta.len() <= 10 * 1024 * 1024 {
        return;
    }

    let wasm_opt = option_env!("WASM_OPT").unwrap_or("wasm-opt");
    let status = Command::new(wasm_opt)
        .args(["-Oz", "--strip-debug"])
        .arg(wasm_path)
        .arg("-o")
        .arg(wasm_path)
        .status();

    match status {
        Ok(s) if s.success() => {
            let new_size = std::fs::metadata(wasm_path)
                .map(|m| m.len() / (1024 * 1024))
                .unwrap_or(0);
            println!(
                "cargo::warning=wasm-opt: optimized freq-ai_bg.wasm to {}MB",
                new_size
            );
        }
        Ok(s) => println!("cargo::warning=wasm-opt exited with {s}, skipping optimization"),
        Err(_) => println!("cargo::warning=wasm-opt not found, skipping wasm optimization"),
    }
}
