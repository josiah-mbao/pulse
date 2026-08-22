use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR environment variable set");
    let dest_path = Path::new(&out_dir).join("pulse-ebpf");

    // Path to built eBPF bytecode relative to crate root
    let ebpf_path = Path::new("target/bpfel-unknown-none/release/pulse-ebpf");

    if ebpf_path.exists() {
        println!("cargo:rerun-if-changed=target/bpfel-unknown-none/release/pulse-ebpf");
        if let Err(e) = fs::copy(ebpf_path, &dest_path) {
            println!("cargo:warning=Failed to copy eBPF bytecode: {}", e);
            let _ = fs::write(&dest_path, []);
        }
    } else {
        // Write dummy empty file so include_bytes! in ebpf_collector.rs always succeeds at compile-time
        let _ = fs::write(&dest_path, []);
    }
}
