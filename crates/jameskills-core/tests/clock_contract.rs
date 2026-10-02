use jameskills_core::ports::ClockPort;
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

struct FakeClock {
    utc: &'static str,
    monotonic: AtomicU64,
}

impl ClockPort for FakeClock {
    fn now_utc(&self) -> String {
        self.utc.to_owned()
    }

    fn monotonic_ms(&self) -> u64 {
        self.monotonic.fetch_add(1, Ordering::SeqCst)
    }
}

fn accepts_shared_clock(_: Arc<dyn ClockPort>) {}

#[test]
fn clock_port_is_object_safe_and_returns_deterministic_fake_values() {
    let clock: Arc<dyn ClockPort> = Arc::new(FakeClock {
        utc: "2026-10-02T12:00:00Z",
        monotonic: AtomicU64::new(41),
    });

    assert_eq!(clock.now_utc(), "2026-10-02T12:00:00Z");
    assert_eq!(clock.monotonic_ms(), 41);
    assert_eq!(clock.monotonic_ms(), 42);
    accepts_shared_clock(clock);
}
