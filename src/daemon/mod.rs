mod ipc;
mod scanner;

use crate::protocol::Project;
use arc_swap::ArcSwap;
use notify::{EventKind, RecursiveMode, Watcher};
use std::os::unix::net::UnixListener;
use std::path::Path;
use std::sync::Arc;
use std::thread;

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

    let projects_dir = format!("{}/projects/", home);

    // Initial scan
    do_scan_projects(&projects, &projects_dir);
    do_scan_apps(&apps);

    // Watcher thread
    {
        let projects_clone = Arc::clone(&projects);
        let apps_clone = Arc::clone(&apps);
        let projects_dir_clone = projects_dir.clone();

        thread::spawn(move || {
            let (tx, rx) = std::sync::mpsc::channel();

            let mut watcher = notify::RecommendedWatcher::new(tx, notify::Config::default())
                .expect("Failed to create filesystem watcher");

            // Watch ~/projects/ for new/removed project directories
            if Path::new(&projects_dir_clone).exists() {
                if let Err(e) =
                    watcher.watch(Path::new(&projects_dir_clone), RecursiveMode::NonRecursive)
                {
                    eprintln!("Failed to watch projects dir: {}", e);
                }
            }

            // Watch app directories for new/removed binaries
            for dir in ["/usr/bin", "/bin"] {
                if Path::new(dir).exists() {
                    if let Err(e) = watcher.watch(Path::new(dir), RecursiveMode::NonRecursive) {
                        eprintln!("Failed to watch {}: {}", dir, e);
                    }
                }
            }

            // Event loop — re-scan on relevant changes
            for event in rx {
                match event {
                    Ok(event) => match event.kind {
                        EventKind::Create(_) | EventKind::Remove(_) | EventKind::Modify(_) => {
                            let is_project_event = event.paths.iter().any(|p| {
                                p.starts_with(&projects_dir_clone)
                            });

                            if is_project_event {
                                do_scan_projects(&projects_clone, &projects_dir_clone);
                            } else {
                                do_scan_apps(&apps_clone);
                            }
                        }
                        _ => {}
                    },
                    Err(e) => eprintln!("Watch error: {}", e),
                }
            }
        });
    }

    (projects, apps)
}

fn do_scan_projects(store: &Arc<ArcSwap<Vec<Project>>>, projects_dir: &str) {
    let mut new_projects = Vec::with_capacity(ESTIMATED_PROJECTS);
    if let Err(err) = scanner::scan_projects(&mut new_projects, projects_dir) {
        eprintln!("Project scan error: {}", err);
    }
    store.store(Arc::new(new_projects));
}

fn do_scan_apps(store: &Arc<ArcSwap<Vec<scanner::App>>>) {
    let mut new_apps = Vec::with_capacity(ESTIMATED_APPS);
    if let Err(err) = scanner::scan_apps(&mut new_apps) {
        eprintln!("App scan error: {}", err);
    }
    store.store(Arc::new(new_apps));
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
