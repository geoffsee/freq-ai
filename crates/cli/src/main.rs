//! Standalone freq-ai binary. The real entry point lives in `lib.rs` so
//! library consumers (e.g. project-specific shims that want to inject custom
//! `Config` fields) can call [`freq_ai::run_with_overrides`] directly.

fn main() {
    freq_ai::run();
}
