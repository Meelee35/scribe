use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use directories::ProjectDirs;

const APP_NAME: &str = "scribe";
const FILE_NAME: &str = "data.json";

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Data {
    pub notes: Vec<Note>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Note {
    pub name: String,
    pub body: String,
    pub todos: Vec<Todo>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Todo {
    pub text: String,
    pub done: bool,
    pub pinned: bool
}

fn data_path() -> PathBuf {
    ProjectDirs::from("io.github", "meelees", APP_NAME).unwrap().data_dir().to_owned()
}
pub fn load() -> anyhow::Result<Data> {
    let dir = data_path();
    let data_path = dir.join("data.json");
    if !data_path.exists() {
        return Ok(Data { notes: Vec::new() });
    }
    let file = fs::File::open(&data_path)?;
    let data: Data = serde_json::from_reader(file)?;
    Ok(data)
}

pub fn save(data: &Data) -> anyhow::Result<()> {
    let dir = data_path();
    fs::create_dir_all(dir.clone())?;
    let file = fs::File::create(dir.join(FILE_NAME))?;
    serde_json::to_writer_pretty(file, &data)?;
    Ok(())
}