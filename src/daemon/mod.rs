mod ipc;
mod scanner;

use crate::protocol::Project;
use arc_swap::ArcSwap;
use std::os::unix::net::UnixListener;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const ESTIMATED_PROJECTS: usize = 50;
const ESTIMATED_APPS: usize = 3000;

pub fn start_daemon(
    home: String,
) -> (
    Arc<ArcSwap<Vec<Project>>>,
    Arc<ArcSwap<Vec<scanner::App>>>,
) {
    let projects = Arc::new(ArcSwap::from_pointee(Vec::new()));
    let apps = Arc::new(ArcSwap::from_pointee(Vec::new()));

    // Project scanner thread — periodic rescan
    {
        let projects_clone = Arc::clone(&projects);
        let home_clone = home.clone();

        thread::spawn(move || {
            loop {
                let mut new_projects = Vec::with_capacity(ESTIMATED_PROJECTS);

                if let Err(err) = scanner::scan_projects(&mut new_projects, &home_clone) {
                    eprintln!("Project scan error: {}", err);
                }

                projects_clone.store(Arc::new(new_projects));

                thread::sleep(Duration::from_secs(5));
            }
        });
    }

    // App scanner — scan once at startup (system binaries rarely change)
    {
        let apps_clone = Arc::clone(&apps);

        thread::spawn(move || {
            let mut new_apps = Vec::with_capacity(ESTIMATED_APPS);

            if let Err(err) = scanner::scan_apps(&mut new_apps) {
                eprintln!("App scan error: {}", err);
            }

            apps_clone.store(Arc::new(new_apps));
        });
    }

    (projects, apps)
}

fn start_ipc_server(projects: Arc<ArcSwap<Vec<Project>>>) {
    let socket_path = "/tmp/mydaemon.sock";

    let _ = std::fs::remove_file(socket_path);

    // TODO: Fix unwrap - handle errors
    let listener = UnixListener::bind(socket_path).unwrap();

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    let projects = Arc::clone(&projects);

                    std::thread::spawn(move || {
                        ipc::handle_client(&mut stream, projects);
                    });
                }
                Err(e) => eprintln!("IPC error: {}", e),
            }
        }
    });
}
