use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::{Arc, RwLock};

use super::Project;
use crate::protocol::{ProjectDto, Request, Response};

pub fn handle_client(stream: &mut UnixStream, projects: Arc<RwLock<Vec<Project>>>) {
    let mut buffer = String::new();

    if let Err(e) = stream.read_to_string(&mut buffer) {
        let _ = stream.write_all(
            serde_json::to_string(&Response::Error(e.to_string()))
                .unwrap()
                .as_bytes(),
        );
        return;
    }

    let request: Request = match serde_json::from_str(&buffer) {
        Ok(req) => req,
        Err(e) => {
            let _ = stream.write_all(
                serde_json::to_string(&Response::Error(e.to_string()))
                    .unwrap()
                    .as_bytes(),
            );
            return;
        }
    };

    let response = match request {
        Request::GetProjects => {
            let pr = projects.read().unwrap();

            let dto: Vec<ProjectDto> = pr
                .iter()
                .map(|p| ProjectDto {
                    name: p.name.clone(),
                    path: p.path.clone(),
                })
                .collect();

            Response::Projects(dto)
        }
    };

    let response_str = serde_json::to_string(&response).unwrap();
    let _ = stream.write_all(response_str.as_bytes());
}
