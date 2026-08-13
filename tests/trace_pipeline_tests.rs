use pulse::system::model::{EventSender, SystemEvent};
use pulse::tui::app::{AppState, TraceEventKind};
use pulse_common::{EVENT_EXEC, EVENT_EXIT, TraceEvent};
use std::sync::mpsc;

#[test]
fn test_trace_pipeline_event_sender_backpressure_drop_accounting() {
    let (tx, _rx) = mpsc::sync_channel(1); // 1-slot channel
    let sender = EventSender::new(tx);

    let sample_trace = SystemEvent::Trace(TraceEvent {
        pid: 100,
        event_type: EVENT_EXEC,
        comm: *b"test-proc\0\0\0\0\0\0\0",
    });

    // Send 1st event -> occupies the 1 slot
    sender.send(sample_trace.clone());
    assert_eq!(sender.dropped_traces(), 0);

    // Send 500 additional events -> backpressure drops them non-blockingly
    for _ in 0..500 {
        sender.send(sample_trace.clone());
    }

    assert_eq!(sender.dropped_traces(), 500);
}

#[test]
fn test_trace_pipeline_app_state_capacity_eviction() {
    let mut app = AppState::new();

    // Ingest 1,000 trace events with PIDs 0..1000
    for i in 0..1000 {
        let evt = TraceEvent {
            pid: i as u32,
            event_type: if i % 2 == 0 { EVENT_EXEC } else { EVENT_EXIT },
            comm: *b"worker-task\0\0\0\0\0",
        };
        app.apply_trace(evt);
    }

    // Capacity cap at 500 must be enforced
    assert_eq!(app.trace_log.len(), 500);

    // FIFO eviction: front should be item 500, back should be item 999
    let front = app.trace_log.front().expect("Front item present");
    let back = app.trace_log.back().expect("Back item present");

    assert_eq!(front.pid, 500);
    assert_eq!(front.kind, TraceEventKind::Exec);
    assert_eq!(back.pid, 999);
    assert_eq!(back.kind, TraceEventKind::Exit);
}

#[test]
fn test_trace_pipeline_pause_resume_behavior() {
    let mut app = AppState::new();

    let evt1 = TraceEvent {
        pid: 111,
        event_type: EVENT_EXEC,
        comm: *b"proc-111\0\0\0\0\0\0\0\0",
    };
    let evt2 = TraceEvent {
        pid: 222,
        event_type: EVENT_EXEC,
        comm: *b"proc-222\0\0\0\0\0\0\0\0",
    };

    // 1. Unpaused -> accepts event
    app.apply_trace(evt1);
    assert_eq!(app.trace_log.len(), 1);
    assert_eq!(app.trace_log.back().unwrap().pid, 111);

    // 2. Paused -> ignores event
    app.paused = true;
    app.apply_trace(evt2);
    assert_eq!(app.trace_log.len(), 1);
    assert_eq!(app.trace_log.back().unwrap().pid, 111);

    // 3. Resumed -> accepts subsequent event
    app.paused = false;
    let evt3 = TraceEvent {
        pid: 333,
        event_type: EVENT_EXIT,
        comm: *b"proc-333\0\0\0\0\0\0\0\0",
    };
    app.apply_trace(evt3);
    assert_eq!(app.trace_log.len(), 2);
    assert_eq!(app.trace_log.back().unwrap().pid, 333);
}
