use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum Request {
    GetProjects,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Response {
    Projects(Vec<ProjectDto>),
    Error(String),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProjectDto {
    pub name: String,
    pub path: String,
}
