pub mod daemon;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::thread;
use std::time::Duration;

fn main() {
    env_logger::init();

    let home = std::env::var("HOME").expect("Failed to get HOME directory");

    // Start daemon (returns both projects and apps)
    let (_projects, _apps) = daemon::start_daemon(home);

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(100));
    }
}
