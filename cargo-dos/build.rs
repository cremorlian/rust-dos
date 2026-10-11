fn main() {
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR not set");
    let out_path = format!("{out_dir}/stub.bin");
    let asm_path = "assets/stub.asm";

    println!("cargo:rerun-if-changed={asm_path}");

    let status = std::process::Command::new("nasm")
        .args(["-f", "bin", asm_path, "-o", &out_path])
        .status()
        .expect("failed to run nasm");

    if !status.success() {
        panic!("nasm failed to assemble {asm_path}");
    }
}
