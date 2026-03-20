use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;
use walkdir::WalkDir;

#[derive(Debug)]

pub struct Project {
    id: u8,
    pub name: String,
    path: String,
}

pub struct App {
    id: u8,
    pub name: String,
    path: String,
}

pub fn start_daemon(home: String) -> (Arc<RwLock<Vec<Project>>>, Arc<RwLock<Vec<App>>>) {
    let projects = Arc::new(RwLock::new(Vec::new()));
    let apps = Arc::new(RwLock::new(Vec::new()));

    // Project scanner thread
    {
        let projects_clone = Arc::clone(&projects);
        let home_clone = home.clone();

        thread::spawn(move || {
            loop {
                let mut new_projects = Vec::new();

                if let Err(err) = scan_projects(&mut new_projects, &home_clone) {
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

                if let Err(err) = scan_apps(&mut new_apps) {
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
fn scan_projects(projects: &mut Vec<Project>, home: &str) -> Result<(), String> {
    use std::path::Path;
    use walkdir::WalkDir;

    let projects_dir = format!("{}/projects/", home);

    if !Path::new(&projects_dir).exists() {
        return Err("Projects directory not found".into());
    }

    for entry in WalkDir::new(&projects_dir).max_depth(1) {
        match entry {
            Ok(entry) => {
                let path = entry.path();

                if path == Path::new(&projects_dir) {
                    continue;
                }

                if path.is_dir() {
                    let path_str = path.to_string_lossy().to_string();

                    projects.push(Project {
                        id: projects.len() as u8,
                        name: path_str
                            .strip_prefix(&projects_dir)
                            .unwrap_or(&path_str)
                            .to_string(),
                        path: path_str,
                    });
                }
            }
            Err(e) => eprintln!("Walk error: {}", e),
        }
    }

    Ok(())
}

fn scan_apps(apps: &mut Vec<App>) -> Result<(), String> {
    use walkdir::WalkDir;

    let paths = vec!["/usr/bin", "/bin"];

    for base in paths {
        for entry in WalkDir::new(base).max_depth(1) {
            match entry {
                Ok(entry) => {
                    let path = entry.path();

                    if path.is_file() {
                        let name = path.file_name().unwrap().to_string_lossy().to_string();

                        apps.push(App {
                            id: apps.len() as u8,
                            name,
                            path: path.to_string_lossy().to_string(),
                        });
                    }
                }
                Err(e) => eprintln!("App scan error: {}", e),
            }
        }
    }

    Ok(())
}
