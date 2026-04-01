use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::Arc;

use arc_swap::ArcSwap;
use rkyv::util::AlignedVec;

use crate::protocol::{Project, Request, Response};

pub fn handle_client(stream: &mut UnixStream, projects: Arc<ArcSwap<Vec<Project>>>) {
    let mut buffer = Vec::new();

    if let Err(e) = stream.read_to_end(&mut buffer) {
        write_error(stream, &e.to_string());
        return;
    }

    // rkyv requires aligned data for deserialization
    let mut aligned: AlignedVec<16> = AlignedVec::new();
    aligned.extend_from_slice(&buffer);

    let request: Request = match rkyv::from_bytes::<Request, rkyv::rancor::Error>(&aligned) {
        Ok(req) => req,
        Err(e) => {
            write_error(stream, &e.to_string());
            return;
        }
    };

    let response = match request {
        Request::GetProjects => {
            let guard = projects.load();
            Response::Projects(guard.to_vec())
        }
    };

    if let Ok(bytes) = rkyv::to_bytes::<rkyv::rancor::Error>(&response) {
        let _ = stream.write_all(&bytes);
    }
}

fn write_error(stream: &mut UnixStream, msg: &str) {
    let response = Response::Error(msg.to_string());
    if let Ok(bytes) = rkyv::to_bytes::<rkyv::rancor::Error>(&response) {
        let _ = stream.write_all(&bytes);
    }
}
