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
    let n = width - u16::try_from(line.len()).expect("Invalid length");
    let blank_space = " ".repeat(n as usize);
    write!(stderr, "{color}{line}\x1b[0m{blank_space}")?;
    Ok(())
}

fn write_to_file(file_path: &str, content: &str) -> io::Result<()> {
    let mut file = std::fs::File::create(file_path)?;
    file.write_all(content.as_bytes())?;
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
    let mut sorted_entries: Vec<_> = entries.collect();
    sorted_entries.sort_by(|a, b| {
        let a_name = a.as_ref().expect("Failed to get entry").file_name();
        let b_name = b.as_ref().expect("Failed to get entry").file_name();
        a_name.cmp(&b_name)
    });

    for entry in sorted_entries {
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
    permission_denied: bool,
    width: u16,
    height: u16,
) -> io::Result<()> {
    if permission_denied {
        for i in 0..height {
            let i = i32::from(i);

            let x = x + 1;
            let y = u16::try_from(i).expect("Invalid index") + 1 + y;
            print_width(stderr, x, y, width, "\x1b[0m", "")?;
        }

        print_width(stderr, x + 1, 3, width, "\x1b[1;31m", "Permission Denied")?;
        return Ok(());
    }

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

fn file_stdout(file_name: &str) -> String {
    let cmd = "file";
    let output = std::process::Command::new(cmd)
        .arg(file_name)
        .output()
        .expect("Failed to execute command");
    let output_str = String::from_utf8_lossy(&output.stdout);
    let output_str = output_str.trim();
    output_str.to_string()
}

fn render(stderr: &mut dyn Write, index: i32) -> io::Result<()> {
    let current_dir = get_cwd();

    let file_names = get_file_names(".")?;
    let parent_file_names = if current_dir == "/" {
        vec![]
    } else {
        get_file_names("..")?
    };

    let (width, height) = terminal_size()?;

    let child_file_names = get_file_names(
        file_names[usize::try_from(index).expect("Invalid index")]
            .path
            .to_str()
            .unwrap_or("."),
    );

    let pane_width = width / 3;

    let filename = file_names
        .get(usize::try_from(index).expect("Invalid index"))
        .map_or("..", |f| f.name.as_str());
    print_width(stderr, 1, 2, width, "", &file_stdout(filename))?;

    print_width(stderr, 1, 1, width, "\x1b[1;32m", get_cwd().as_str())?;
    render_pane(
        stderr,
        0,
        2,
        -1,
        &parent_file_names,
        false,
        pane_width,
        height - 1,
    )?;
    render_pane(
        stderr,
        width / 3,
        2,
        index,
        &file_names,
        false,
        pane_width,
        height - 1,
    )?;
    render_pane(
        stderr,
        2 * width / 3,
        2,
        -1,
        child_file_names.as_ref().unwrap_or(&vec![]),
        child_file_names.is_err(),
        pane_width,
        height - 1,
    )?;

    stderr.flush()?;
    Ok(())
}

fn main() -> Result<(), io::Error> {
    let mut stderr = io::stderr().into_raw_mode()?;
    let mut index = 0;
    write!(
        stderr,
        "\x1b[?1049h{}{}{}",
        clear::All,
        cursor::Hide,
        cursor::Goto(1, 1)
    )?;

    render(&mut stderr, index)?;

    for byte in io::stdin().bytes() {
        let current_dir_files = get_file_names(".")?;
        match byte? {
            b'q' => break,
            b'j' => index += 1,
            b'k' => index -= 1,
            b'l' => {
                if index >= 0 && index < current_dir_files.len().try_into().expect("Invalid index")
                {
                    try_cd(
                        &current_dir_files[usize::try_from(index).expect("Invalid index")].path,
                    )?;
                } else {
                    index = 0;
                }
            }
            b'h' => {
                let cwd = get_cwd();
                let dirname = cwd.split('/').collect::<Vec<_>>();
                let old_dir = dirname.last().expect("Failed to get last directory");
                try_cd(&PathBuf::from(".."))?;
                let files = get_file_names(".")?;
                index = 0;
                for (i, _) in files.iter().enumerate() {
                    if files[i].name == *old_dir {
                        index = i32::try_from(i).expect("Invalid index");
                        break;
                    }
                }
            }
            _ => {}
        }

        if index < 0 {
            index = 0;
        }

        let current_dir_files = get_file_names(".")?;
        if index >= i32::try_from(current_dir_files.len()).expect("Invalid index") {
            index = i32::try_from(current_dir_files.len()).expect("Invalid index") - 1;
        }

        render(&mut stderr, index)?;
    }

    write!(stderr, "{}{}\x1b[?1049l", cursor::Show, clear::All)?;
    stderr.flush()?;

    write_to_file("/tmp/directory-switcher", get_cwd().as_str())?;
    Ok(())
}
