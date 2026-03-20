use crate::protocol;

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

pub fn scan_projects(projects: &mut Vec<Project>, home: &str) -> Result<(), String> {
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

pub fn scan_apps(apps: &mut Vec<App>) -> Result<(), String> {
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
