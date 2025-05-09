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

fn render_pane(
    stderr: &mut dyn Write,
    x: u16,
    y: u16,
    index: i32,
    file_names: &Vec<FileInfo>,
    width: u16,
) -> io::Result<()> {
    for (i, file_info) in file_names.iter().enumerate() {
        write!(stderr, "{}", cursor::Goto(1 + x, i as u16 + 1 + y))?;
        if index == i as i32 {
            write!(stderr, "\x1b[7m")?;
        }

        if file_info.is_dir {
            write!(stderr, "\x1b[34m")?;
        }

        let mut line = file_info.name.clone();
        line.truncate(width as usize);
        write!(stderr, "{}", line)?;
        write!(stderr, "\x1b[0m")?;
    }

    Ok(())
}

fn render(
    stderr: &mut dyn Write,
    index: i32,
    parent_file_names: &Vec<FileInfo>,
    file_names: &Vec<FileInfo>,
) -> io::Result<()> {
    let (width, height) = terminal_size()?;
    write!(stderr, "{}", clear::All)?;

    let child_file_names = if index >= 0
        && index < file_names.len().try_into().unwrap()
        && file_names[index as usize].is_dir
    {
        get_file_names(file_names[index as usize].path.to_str().unwrap_or("."))?
    } else {
        Vec::new()
    };

    let pane_width = width / 3;

    render_pane(stderr, 0, 1, -1, parent_file_names, pane_width)?;
    render_pane(stderr, width / 3, 1, index, file_names, pane_width)?;
    render_pane(stderr, 2 * width / 3, 1, -1, &child_file_names, pane_width)?;

    stderr.flush()?;
    Ok(())
}

fn main() -> Result<(), io::Error> {
    let mut stderr = io::stderr().into_raw_mode()?;
    write!(stderr, "\x1b[?1049h")?;
    let mut index = 0;
    write!(
        stderr,
        "{}{}{}",
        clear::All,
        cursor::Hide,
        cursor::Goto(1, 1)
    )?;

    let mut current_dir_files = get_file_names(".")?;
    let mut parent_dir_files = get_file_names("..")?;
    render(&mut stderr, index, &parent_dir_files, &current_dir_files)?;

    for byte in io::stdin().bytes() {
        match byte? {
            b'q' => break,
            b'j' => index += 1,
            b'k' => index -= 1,
            b'l' => {
                if index >= 0 && index < current_dir_files.len().try_into().unwrap() {
                    try_cd(&current_dir_files[index as usize].path)?;
                }
                index = 0;
            }
            b'h' => {
                try_cd(&PathBuf::from(".."))?;
                index = 0;
            }
            _ => {}
        }

        if index < 0 {
            index = 0;
        }

        current_dir_files = get_file_names(".")?;
        if index >= current_dir_files.len() as i32 {
            index = current_dir_files.len() as i32 - 1;
        }

        parent_dir_files = get_file_names("..")?;
        render(&mut stderr, index, &parent_dir_files, &current_dir_files)?;
    }

    write!(stderr, "{}{}", cursor::Show, clear::All)?;
    write!(stderr, "\x1b[?1049l")?;
    stderr.flush()?;

    write!(stderr, "Current directory: {}\n", get_cwd())?;

    Ok(())
}
