use std::io::{self, Read, Write};
use std::path::PathBuf;
use termion::{clear, cursor, raw::IntoRawMode, terminal_size};

struct FileInfo {
    name: String,
    is_dir: bool,
    path: PathBuf,
}

fn get_cwd() -> String {
    let mut path = PathBuf::new();
    if let Ok(current_dir) = std::env::current_dir() {
        path = current_dir;
    }
    path.to_str().unwrap_or(".").to_string()
}

fn try_cd(path: &PathBuf) -> io::Result<()> {
    if path.is_dir() {
        std::env::set_current_dir(path)?;
    }
    Ok(())
}

fn get_file_names(directory: &str) -> io::Result<Vec<FileInfo>> {
    let mut file_names = Vec::new();

    let dir_path = PathBuf::from(directory);
    if !dir_path.is_dir() {
        return Ok(file_names);
    }

    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();

        if let Some(name) = file_name.to_str() {
            file_names.push(FileInfo {
                name: name.to_string(),
                is_dir: path.is_dir(),
                path: path.clone(),
            });
        }
    }
    Ok(file_names)
}

fn main() -> Result<(), io::Error> {
}
