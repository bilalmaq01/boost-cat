#![cfg_attr(windows, windows_subsystem = "windows")]
#[cfg(windows)]
mod win;
fn main() {
    #[cfg(windows)]
    {
        win::run();
    }
    #[cfg(not(windows))]
    println!("Boost Cat is a native Windows application. Run cargo test to check pricing rules.");
}
