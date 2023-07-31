use std::process::{Command, Stdio};

fn main() {
    println!("cargo:rerun-if-changed=../client/src");
    println!("cargo:rerun-if-changed=../client/Cargo.toml");

    let is_release =
        std::env::var("PROFILE").expect("$PROFILE should be defined by cargo") == "release";
    let mut out_dir = std::path::PathBuf::from(
        std::env::var("OUT_DIR").expect("$PROFILE should be defined by cargo"),
    );
    out_dir.push("client_subbuild");
    let out_dir_str = out_dir
        .to_str()
        .expect("paths should probably be valid utf8");

    let mut wasm_build = Command::new("cargo");
    wasm_build
        .current_dir("../client")
        .arg("build")
        .arg("--target=wasm32-unknown-unknown")
        .arg(format!("--target-dir={}", out_dir_str));
    if is_release {
        wasm_build.arg("--release");
    }
    let exit_code = wasm_build
        .stdin(Stdio::null())
        .status()
        .expect("failed to start client frontend build");

    if !exit_code.success() {
        std::process::exit(exit_code.code().unwrap_or(1))
    }

    let mut wasm_bindgen = Command::new("wasm-bindgen");
    wasm_bindgen
        .current_dir("../client")
        .arg("--target=web")
        .arg("--weak-refs")
        .arg("--reference-types")
        .arg("--no-typescript")
        .arg(format!("--out-dir={out_dir_str}/wasm-bindgen"));
    if is_release {
        wasm_bindgen.arg(format!(
            "{out_dir_str}/wasm32-unknown-unknown/release/moose2_client.wasm",
        ));
    } else {
        wasm_bindgen.arg(format!(
            "{out_dir_str}/wasm32-unknown-unknown/debug/moose2_client.wasm",
        ));
    }
    wasm_bindgen
        .status()
        .expect("failed to start client frontend build");

    if !exit_code.success() {
        std::process::exit(exit_code.code().unwrap_or(1))
    }
}
