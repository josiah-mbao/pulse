use pulse::tui::app::{TraceEventKind, TraceEventView};
use pulse_common::{EVENT_EXEC, EVENT_EXIT, TraceEvent};

#[test]
fn test_ebpf_trace_event_unaligned_memory_decoding() {
    // Construct unaligned raw byte buffer containing serialized TraceEvent
    let event = TraceEvent {
        pid: 9876,
        event_type: EVENT_EXEC,
        comm: *b"sample-comm\0\0\0\0\0",
    };

    let size = std::mem::size_of::<TraceEvent>();
    let mut bytes = vec![0u8; size + 8]; // padded byte buffer

    // Place TraceEvent at an unaligned odd offset (+3)
    let offset = 3;
    unsafe {
        std::ptr::copy_nonoverlapping(
            &event as *const TraceEvent as *const u8,
            bytes.as_mut_ptr().add(offset),
            size,
        );
    }

    // Read back using unaligned pointer read (mimicking Aya RingBuf entry parsing)
    let unaligned_ptr = unsafe { bytes.as_ptr().add(offset) as *const TraceEvent };
    let decoded: TraceEvent = unsafe { std::ptr::read_unaligned(unaligned_ptr) };

    assert_eq!(decoded.pid, 9876);
    assert_eq!(decoded.event_type, EVENT_EXEC);
    assert_eq!(&decoded.comm, b"sample-comm\0\0\0\0\0");
}

#[test]
fn test_ebpf_trace_event_view_utf8_lossy_and_null_trimming() {
    // Case 1: Comm with invalid UTF-8 bytes and trailing nulls
    let invalid_utf8_comm = [
        0xFF, 0xFE, 0x80, b'b', b'a', b's', b'h', 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let event1 = TraceEvent {
        pid: 1234,
        event_type: EVENT_EXEC,
        comm: invalid_utf8_comm,
    };
    let view1 = TraceEventView::from(event1);
    assert_eq!(view1.pid, 1234);
    assert_eq!(view1.kind, TraceEventKind::Exec);
    assert!(view1.comm.contains("bash"));

    // Case 2: Comm without null-terminator (full 16-byte ascii string)
    let full_comm = *b"0123456789abcdef";
    let event2 = TraceEvent {
        pid: 5678,
        event_type: EVENT_EXIT,
        comm: full_comm,
    };
    let view2 = TraceEventView::from(event2);
    assert_eq!(view2.pid, 5678);
    assert_eq!(view2.kind, TraceEventKind::Exit);
    assert_eq!(view2.comm, "0123456789abcdef");

    // Case 3: All-zero comm array
    let zero_comm = [0u8; 16];
    let event3 = TraceEvent {
        pid: 1,
        event_type: EVENT_EXEC,
        comm: zero_comm,
    };
    let view3 = TraceEventView::from(event3);
    assert_eq!(view3.pid, 1);
    assert_eq!(view3.comm, "");
}

#[test]
fn test_ebpf_trace_event_type_classification() {
    let exec_evt = TraceEvent {
        pid: 10,
        event_type: EVENT_EXEC,
        comm: *b"exec-proc\0\0\0\0\0\0\0",
    };
    let exit_evt = TraceEvent {
        pid: 20,
        event_type: EVENT_EXIT,
        comm: *b"exit-proc\0\0\0\0\0\0\0",
    };
    let unknown_evt = TraceEvent {
        pid: 30,
        event_type: 999, // Unknown integer type
        comm: *b"unknown-proc\0\0\0\0",
    };

    assert_eq!(TraceEventView::from(exec_evt).kind, TraceEventKind::Exec);
    assert_eq!(TraceEventView::from(exit_evt).kind, TraceEventKind::Exit);
    // Unknown types default safely to Exit (non-Exec) without panic
    assert_eq!(TraceEventView::from(unknown_evt).kind, TraceEventKind::Exit);
}
