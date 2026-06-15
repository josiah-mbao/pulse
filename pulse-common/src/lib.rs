#![no_std]

pub const EVENT_EXEC: u32 = 0;
pub const EVENT_EXIT: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct TraceEvent {
    pub pid: u32,
    pub event_type: u32,
    pub comm: [u8; 16],
}

#[cfg(feature = "user")]
impl core::fmt::Debug for TraceEvent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let comm = core::str::from_utf8(&self.comm)
            .unwrap_or("unknown")
            .trim_end_matches('\0');
        f.debug_struct("TraceEvent")
            .field("pid", &self.pid)
            .field("event_type", &self.event_type)
            .field("comm", &comm)
            .finish()
    }
}
