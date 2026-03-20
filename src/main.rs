pub mod daemon;
pub mod popup;
pub mod protocol;

use std::thread;
use std::time::Duration;

fn main() {
    let home = std::env::var("HOME").expect("Failed to get HOME directory");

    // Start daemon (now returns BOTH projects and apps)
    let (projects, apps) = daemon::start_daemon(home);

    // Temporary debug loop (replace later with UI event loop)
    loop {
        {
            let pr = projects.read().unwrap();
            let ap = apps.read().unwrap();

            println!("==== CURRENT STATE ====");

            println!("Projects:");
            for p in pr.iter() {
                println!("  {}", p.name);
            }

            println!("Apps:");
            for a in ap.iter() {
                println!("  {}", a.name);
            }
        } // locks released here immediately

        thread::sleep(Duration::from_secs(10));
    }
}
