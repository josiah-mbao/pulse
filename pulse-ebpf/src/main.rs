#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::bpf_get_current_pid_tgid,
    macros::{map, tracepoint},
    maps::RingBuf,
    programs::TracePointContext,
};
use pulse_common::{EVENT_EXEC, EVENT_EXIT, TraceEvent};

#[map]
static EVENTS: RingBuf = RingBuf::with_byte_size(256 * 1024, 0);

#[tracepoint]
pub fn sched_process_exec(ctx: TracePointContext) -> u32 {
    match try_sched_process_exec(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_sched_process_exec(_ctx: TracePointContext) -> Result<u32, u32> {
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    let mut comm = [0u8; 16];
    unsafe {
        aya_ebpf::helpers::r#gen::bpf_get_current_comm(&mut comm as *mut _ as *mut _, 16);
    }

    let event = TraceEvent {
        pid,
        event_type: EVENT_EXEC,
        comm,
    };

    if let Some(mut slot) = EVENTS.reserve::<TraceEvent>(0) {
        unsafe {
            core::ptr::write(slot.as_mut_ptr(), event);
        }
        slot.submit(0);
    }

    Ok(0)
}

#[tracepoint]
pub fn sched_process_exit(ctx: TracePointContext) -> u32 {
    match try_sched_process_exit(ctx) {
        Ok(ret) => ret,
        Err(ret) => ret,
    }
}

fn try_sched_process_exit(_ctx: TracePointContext) -> Result<u32, u32> {
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    let mut comm = [0u8; 16];
    unsafe {
        aya_ebpf::helpers::r#gen::bpf_get_current_comm(&mut comm as *mut _ as *mut _, 16);
    }

    let event = TraceEvent {
        pid,
        event_type: EVENT_EXIT,
        comm,
    };

    if let Some(mut slot) = EVENTS.reserve::<TraceEvent>(0) {
        unsafe {
            core::ptr::write(slot.as_mut_ptr(), event);
        }
        slot.submit(0);
    }

    Ok(0)
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
