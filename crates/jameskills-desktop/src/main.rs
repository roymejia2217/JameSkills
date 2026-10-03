#[path = "views/platform_probe.rs"]
mod platform_probe;

mod composition;
mod theme;

fn main() {
    composition::bootstrap_desktop();
}
