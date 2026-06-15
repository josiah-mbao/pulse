#![no_std]
#![no_main]

use aya_ebpf::macros::kprobe;
use aya_ebpf::programs::ProbeContext;

#[kprobe]
pub fn pulse_ebpf(ctx: ProbeContext) -> u32 {
    match try_pulse_ebpf(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_pulse_ebpf(_ctx: ProbeContext) -> Result<u32, u32> {
    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
