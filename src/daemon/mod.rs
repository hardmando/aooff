mod ipc;
mod scanner;

use crate::protocol::{ProjectDto, Request, Response};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use walkdir::WalkDir;

pub fn start_daemon(
    home: String,
) -> (
    Arc<RwLock<Vec<scanner::Project>>>,
    Arc<RwLock<Vec<scanner::App>>>,
) {
    let projects = Arc::new(RwLock::new(Vec::new()));
    let apps = Arc::new(RwLock::new(Vec::new()));

    // Project scanner thread
    {
        let projects_clone = Arc::clone(&projects);
        let home_clone = home.clone();

        thread::spawn(move || {
            loop {
                let mut new_projects = Vec::new();

                if let Err(err) = scanner::scan_projects(&mut new_projects, &home_clone) {
                    eprintln!("Project scan error: {}", err);
                }

                {
                    let mut pr = projects_clone.write().unwrap();
                    *pr = new_projects; // fast swap
                }

                thread::sleep(Duration::from_secs(5));
            }
        });
    }

    // App scanner thread
    {
        let apps_clone = Arc::clone(&apps);

        thread::spawn(move || {
            loop {
                let mut new_apps = Vec::new();

                if let Err(err) = scanner::scan_apps(&mut new_apps) {
                    eprintln!("App scan error: {}", err);
                }

                {
                    let mut ap = apps_clone.write().unwrap();
                    *ap = new_apps; // fast swap
                }

                thread::sleep(Duration::from_secs(10));
            }
        });
    }

    (projects, apps)
}

fn start_ipc_server(projects: Arc<RwLock<Vec<scanner::Project>>>) {
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
