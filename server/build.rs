use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=../client/src");
    println!("cargo:rerun-if-changed=../client/Cargo.toml");
    let mut wasm_build = Command::new("cargo");
    wasm_build
        .current_dir("../client")
        .arg("build")
        .arg("--target=wasm32-unknown-unknown");
    #[cfg(not(debug_assertions))]
    wasm_build.arg("--release");
    wasm_build
        .spawn()
        .expect("failed to start client frontend build");

    let mut wasm_bindgen = Command::new("wasm-bindgen");
    wasm_bindgen
        .current_dir("../client")
        .arg("--target=web")
        .arg("--weak-refs")
        .arg("--reference-types")
        .arg("--no-typescript");
    #[cfg(not(debug_assertions))]
    wasm_bindgen
        .arg("--out-dir=./target/wasm-bindgen/release")
        .arg("target/wasm32-unknown-unknown/release/moose2_client.wasm");
    #[cfg(debug_assertions)]
    wasm_bindgen
        .arg("--out-dir=./target/wasm-bindgen/debug")
        .arg("target/wasm32-unknown-unknown/debug/moose2_client.wasm");
    wasm_bindgen
        .spawn()
        .expect("failed to start client frontend build");
}
