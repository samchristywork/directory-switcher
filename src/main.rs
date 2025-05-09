use std::io::{self, Read, Write};
use std::path::PathBuf;
use termion::{clear, cursor, raw::IntoRawMode, terminal_size};

struct FileInfo {
    name: String,
    color: String,
    path: PathBuf,
}

fn print_width(
    stderr: &mut dyn Write,
    x: u16,
    y: u16,
    width: u16,
    color: &str,
    content: &str,
) -> io::Result<()> {
    write!(stderr, "{}", cursor::Goto(x, y))?;
    let mut line = content.to_string();
    line.truncate(width as usize);
    let blank_space = " ".repeat((width - line.len() as u16) as usize);
    write!(stderr, "{}{line}\x1b[0m{}", color, blank_space)?;
    Ok(())
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

    let entries = std::fs::read_dir(directory)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();

        let metadata = entry.metadata()?;
        let color = if metadata.is_dir() {
            "\x1b[1;34m"
        } else if metadata.file_type().is_symlink() {
            "\x1b[1;36m"
        } else {
            "\x1b[1;37m"
        };

        if let Some(name) = file_name.to_str() {
            file_names.push(FileInfo {
                name: name.to_string(),
                color: color.to_string(),
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
    file_names: &[FileInfo],
    width: u16,
    height: u16,
) -> io::Result<()> {
    for i in 0..height {
        let i = i32::from(i);

        let x = x + 1;
        let y = u16::try_from(i).expect("Invalid index") + 1 + y;

        if i >= file_names.len().try_into().expect("Invalid index") {
            print_width(stderr, x, y, width, "\x1b[0m", "")?;
        } else if index == i {
            let file_info = &file_names[usize::try_from(i).expect("Invalid index")];
            print_width(stderr, x, y, width, "\x1b[7m", &file_info.name)?;
        } else {
            let file_info = &file_names[usize::try_from(i).expect("Invalid index")];
            print_width(
                stderr,
                x,
                y,
                width,
                file_info.color.as_str(),
                &file_info.name,
            )?;
        }
    }

    Ok(())
}

fn render(stderr: &mut dyn Write, index: i32) -> io::Result<()> {
    let file_names = get_file_names(".")?;
    let parent_file_names = get_file_names("..")?;

    let (width, height) = terminal_size()?;

    let child_file_names = get_file_names(
        file_names[usize::try_from(index).expect("Invalid index")]
            .path
            .to_str()
            .unwrap_or("."),
    )?;

    let pane_width = width / 3;

    print_width(stderr, 1, 1, width - 2, "\x1b[1;32m", get_cwd().as_str())?;
    render_pane(stderr, 0, 2, -1, &parent_file_names, pane_width, height - 1)?;
    render_pane(
        stderr,
        width / 3,
        2,
        index,
        &file_names,
        pane_width,
        height - 1,
    )?;
    render_pane(
        stderr,
        2 * width / 3,
        2,
        -1,
        &child_file_names,
        pane_width,
        height - 1,
    )?;

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
