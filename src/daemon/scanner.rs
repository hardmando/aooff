use aooff::protocol::{App, Project};

pub fn scan_projects(projects: &mut Vec<Project>, projects_dir: &str) -> Result<(), String> {
    use std::path::Path;
    use walkdir::WalkDir;

    if !Path::new(projects_dir).exists() {
        return Err("Projects directory not found".into());
    }

    for entry in WalkDir::new(projects_dir).max_depth(1) {
        match entry {
            Ok(entry) => {
                let path = entry.path();

                if path == Path::new(projects_dir) {
                    continue;
                }

                if path.is_dir() {
                    let path_str = path.to_string_lossy().to_string();

                    let name: Box<str> = path_str
                        .strip_prefix(projects_dir)
                        .unwrap_or(&path_str)
                        .into();

                    projects.push(Project {
                        name,
                        path: path_str.into_boxed_str(),
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
                        apps.push(App {
                            name: path
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .into_owned()
                                .into_boxed_str(),
                            path: path.to_string_lossy().into_owned().into_boxed_str(),
                        });
                    }
                }
                Err(e) => eprintln!("App scan error: {}", e),
            }
        }
    }

    Ok(())
}
