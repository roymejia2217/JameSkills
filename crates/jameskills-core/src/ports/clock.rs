/// Supplies wall-clock display timestamps and elapsed-time measurements.
///
/// Wall-clock values are for display and audit fields only. Causal ordering and
/// revision identity must come from explicit IDs and parent relationships.
pub trait ClockPort: Send + Sync {
    fn now_utc(&self) -> String;
    fn monotonic_ms(&self) -> u64;
}
