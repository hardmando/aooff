use rkyv::{Archive, Deserialize, Serialize};

#[derive(Archive, Serialize, Deserialize, Debug)]
#[rkyv(derive(Debug))]
pub enum Request {
    GetProjects,
    GetAll,
}

#[derive(Archive, Serialize, Deserialize, Debug)]
#[rkyv(derive(Debug))]
pub enum Response {
    Projects(Vec<Project>),
    All {
        projects: Vec<Project>,
        apps: Vec<App>,
    },
    Error(String),
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
#[rkyv(derive(Debug))]
pub struct Project {
    pub name: Box<str>,
    pub path: Box<str>,
}

#[derive(Archive, Serialize, Deserialize, Debug, Clone)]
#[rkyv(derive(Debug))]
pub struct App {
    pub name: Box<str>,
    pub path: Box<str>,
}
