use pulse::system::ebpf_collector::load_ebpf;

#[test]
#[ignore = "Requires root permissions / CAP_BPF / CAP_PERFMON to load real kernel eBPF bytecode"]
fn test_kernel_ebpf_load_and_attach_privileged() {
    // Attempt loading real eBPF bytecode file target/bpfel-unknown-none/release/pulse-ebpf
    match load_ebpf() {
        Ok(_bpf) => {
            println!("Successfully loaded real eBPF bytecode in privileged environment");
        }
        Err(e) => {
            panic!("Privileged eBPF load failed: {:?}", e);
        }
    }
}
