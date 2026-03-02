use std::cmp::Reverse;
use std::io::{self, BufRead, Read, Write};
use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use termion::{clear, cursor, raw::IntoRawMode, terminal_size};

#[derive(Clone, Copy, PartialEq)]
enum SortMode {
    Name,
    Size,
    Mtime,
}

impl SortMode {
    fn cycle(self) -> Self {
        match self {
            Self::Name  => Self::Size,
            Self::Size  => Self::Mtime,
            Self::Mtime => Self::Name,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Name  => "name",
            Self::Size  => "size",
            Self::Mtime => "mtime",
        }
    }
}

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

fn get_cwd() -> io::Result<String> {
    let path = std::env::current_dir()?;
    Ok(path.to_string_lossy().into_owned())
}

fn try_cd(path: &PathBuf) -> io::Result<()> {
    if path.is_dir() {
        std::env::set_current_dir(path)?;
    }
    Ok(())
}

fn get_file_names(directory: &str, show_hidden: bool, sort_mode: SortMode) -> io::Result<Vec<FileInfo>> {
    let mut file_names = Vec::new();

    let dir_path = PathBuf::from(directory);
    if !dir_path.is_dir() {
        return Ok(file_names);
    }

    let entries = std::fs::read_dir(directory)?;
    let mut sorted_entries: Vec<_> = entries.flatten().collect();
    match sort_mode {
        SortMode::Name  => sorted_entries.sort_by(|a, b| a.file_name().cmp(&b.file_name())),
        SortMode::Size  => sorted_entries.sort_by_key(|e| Reverse(e.metadata().map(|m| m.len()).unwrap_or(0))),
        SortMode::Mtime => sorted_entries.sort_by_key(|e| Reverse(e.metadata().map(|m| m.mtime()).unwrap_or(0))),
    }

    for entry in sorted_entries {
        let path = entry.path();
        let file_name = entry.file_name();

        let Some(name) = file_name.to_str() else {
            continue;
        };

        if !show_hidden && name.starts_with('.') {
            continue;
        }

        let file_type = entry.file_type()?;
        let color = if file_type.is_symlink() {
            "\x1b[1;36m"
        } else if file_type.is_dir() {
            "\x1b[1;34m"
        } else {
            "\x1b[1;37m"
        };

        file_names.push(FileInfo {
            name: name.to_string(),
            color: color.to_string(),
            path: path.clone(),
        });
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

fn read_file_preview(path: &PathBuf, max_lines: usize) -> Vec<String> {
    let Ok(file) = std::fs::File::open(path) else {
        return vec![];
    };
    io::BufReader::new(file)
        .lines()
        .take(max_lines)
        .filter_map(|l| l.ok())
        .collect()
}

fn format_permissions(mode: u32) -> String {
    let ft = match mode & 0o170000 {
        0o040000 => 'd',
        0o120000 => 'l',
        _ => '-',
    };
    let bits = [
        (0o400, 'r'), (0o200, 'w'), (0o100, 'x'),
        (0o040, 'r'), (0o020, 'w'), (0o010, 'x'),
        (0o004, 'r'), (0o002, 'w'), (0o001, 'x'),
    ];
    let mut s = String::with_capacity(10);
    s.push(ft);
    for (bit, ch) in bits {
        s.push(if mode & bit != 0 { ch } else { '-' });
    }
    s
}

fn format_size(bytes: u64) -> String {
    const K: u64 = 1024;
    const M: u64 = K * 1024;
    const G: u64 = M * 1024;
    if bytes >= G {
        format!("{:.1}G", bytes as f64 / G as f64)
    } else if bytes >= M {
        format!("{:.1}M", bytes as f64 / M as f64)
    } else if bytes >= K {
        format!("{:.1}K", bytes as f64 / K as f64)
    } else {
        format!("{bytes}B")
    }
}

fn epoch_days_to_date(mut d: u64) -> (u64, u64, u64) {
    let n400 = d / 146097; d %= 146097;
    let n100 = (d / 36524).min(3); d -= n100 * 36524;
    let n4   = d / 1461;   d %= 1461;
    let n1   = (d / 365).min(3); d -= n1 * 365;
    let year = n400 * 400 + n100 * 100 + n4 * 4 + n1 + 1970;
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let mdays: [u64; 12] = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1u64;
    for md in mdays {
        if d < md { break; }
        d -= md;
        month += 1;
    }
    (year, month, d + 1)
}

fn format_mtime(secs: i64) -> String {
    if secs < 0 { return String::from("?"); }
    let secs = secs as u64;
    let mins  = secs / 60;
    let hours = mins / 60;
    let (y, mo, d) = epoch_days_to_date(hours / 24);
    format!("{y}-{mo:02}-{d:02} {:02}:{:02}", hours % 24, mins % 60)
}

fn file_metadata_str(path: &PathBuf) -> String {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return String::new();
    };
    format!(
        "{}  {}  {}",
        format_permissions(meta.mode()),
        format_size(meta.len()),
        format_mtime(meta.mtime()),
    )
}

fn file_stdout(file_name: &str) -> String {
    let Ok(output) = std::process::Command::new("file").arg(file_name).output() else {
        return String::new();
    };
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn render(stderr: &mut dyn Write, index: i32, show_hidden: bool, filter: &str, filter_mode: bool, sort_mode: SortMode, scroll_offset: i32) -> io::Result<()> {
    let current_dir = get_cwd()?;

    let mut file_names = get_file_names(".", show_hidden, sort_mode)?;
    if !filter.is_empty() {
        let fl = filter.to_lowercase();
        file_names.retain(|f| f.name.to_lowercase().contains(&fl));
    }

    let parent_file_names = if current_dir == "/" {
        vec![]
    } else {
        get_file_names("..", show_hidden, sort_mode)?
    };

    let parent_index = std::path::Path::new(&current_dir)
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|name| {
            parent_file_names
                .iter()
                .position(|f| f.name == name)
                .and_then(|i| i32::try_from(i).ok())
        })
        .unwrap_or(-1);

    let (width, height) = terminal_size()?;
    let pane_width = width / 3;

    let status_line = if filter_mode {
        format!("/{filter}_")
    } else if !filter.is_empty() {
        format!("/{filter}")
    } else {
        String::new()
    };

    let total = file_names.len();
    let count_str = if total == 0 {
        String::from("0/0")
    } else {
        format!("{}/{}", index + 1, total)
    };
    let header = if sort_mode == SortMode::Name {
        format!("{current_dir}  {count_str}")
    } else {
        format!("{current_dir}  [{}]  {count_str}", sort_mode.label())
    };

    let pane_height = (height as i32).saturating_sub(2);

    let par_scroll = if parent_index > 0 {
        ((parent_index - pane_height / 2).max(0)) as usize
    } else {
        0
    };
    let par_files = &parent_file_names[par_scroll.min(parent_file_names.len())..];
    let par_visible_index = parent_index - par_scroll as i32;

    if file_names.is_empty() {
        print_width(stderr, 1, 1, width, "\x1b[1;32m", &header)?;
        print_width(stderr, 1, 2, width, "\x1b[1;33m", &status_line)?;
        render_pane(stderr, 0, 2, par_visible_index, par_files, false, pane_width, height - 1)?;
        render_pane(stderr, width / 3, 2, -1, &[], false, pane_width, height - 1)?;
        render_pane(stderr, 2 * width / 3, 2, -1, &[], false, pane_width, height - 1)?;
        stderr.flush()?;
        return Ok(());
    }

    let mid_scroll = scroll_offset.max(0) as usize;
    let mid_files = &file_names[mid_scroll.min(file_names.len())..];
    let mid_visible_index = index - scroll_offset;

    let selected = &file_names[usize::try_from(index).expect("Invalid index")];
    let is_dir = selected.path.is_dir();

    let child_file_names = if is_dir {
        Some(get_file_names(
            selected.path.to_str().unwrap_or("."),
            show_hidden,
            sort_mode,
        ))
    } else {
        None
    };

    if !status_line.is_empty() {
        print_width(stderr, 1, 2, width, "\x1b[1;33m", &status_line)?;
    } else {
        let meta = file_metadata_str(&selected.path);
        let fout = file_stdout(&selected.name);
        let info = match (meta.is_empty(), fout.is_empty()) {
            (false, false) => format!("{meta}   {fout}"),
            (false, true)  => meta,
            _              => fout,
        };
        print_width(stderr, 1, 2, width, "", &info)?;
    }

    print_width(stderr, 1, 1, width, "\x1b[1;32m", &header)?;
    render_pane(stderr, 0, 2, par_visible_index, par_files, false, pane_width, height - 1)?;
    render_pane(stderr, width / 3, 2, mid_visible_index, mid_files, false, pane_width, height - 1)?;

    match child_file_names {
        Some(Ok(ref entries)) => {
            render_pane(stderr, 2 * width / 3, 2, -1, entries, false, pane_width, height - 1)?;
        }
        Some(Err(_)) => {
            render_pane(stderr, 2 * width / 3, 2, -1, &[], true, pane_width, height - 1)?;
        }
        None => {
            let preview = read_file_preview(&selected.path, (height - 1) as usize);
            for i in 0..(height - 1) {
                let content = preview.get(i as usize).map(|s| s.as_str()).unwrap_or("");
                print_width(stderr, 2 * width / 3 + 1, i + 3, pane_width, "\x1b[0m", content)?;
            }
        }
    }

    stderr.flush()?;
    Ok(())
}

fn main() -> Result<(), io::Error> {
    let mut stderr = io::stderr().into_raw_mode()?;
    let mut index = 0;
    let mut scroll_offset: i32 = 0;
    let mut show_hidden = false;
    let mut filter = String::new();
    let mut filter_mode = false;
    let mut sort_mode = SortMode::Name;
    write!(
        stderr,
        "\x1b[?1049h{}{}{}",
        clear::All,
        cursor::Hide,
        cursor::Goto(1, 1)
    )?;

    render(&mut stderr, index, show_hidden, &filter, filter_mode, sort_mode, scroll_offset)?;

    for byte in io::stdin().bytes() {
        let filtered_files = {
            let mut files = get_file_names(".", show_hidden, sort_mode)?;
            if !filter.is_empty() {
                let fl = filter.to_lowercase();
                files.retain(|f| f.name.to_lowercase().contains(&fl));
            }
            files
        };
        match byte? {
            b'q' if !filter_mode => break,
            b'j' if !filter_mode => index += 1,
            b'k' if !filter_mode => index -= 1,
            b'l' if !filter_mode => {
                if index >= 0 && index < filtered_files.len().try_into().expect("Invalid index") {
                    try_cd(
                        &filtered_files[usize::try_from(index).expect("Invalid index")].path,
                    )?;
                }
                filter.clear();
                index = 0;
                scroll_offset = 0;
            }
            b'h' if !filter_mode => {
                let cwd = get_cwd()?;
                let dirname = cwd.split('/').collect::<Vec<_>>();
                let old_dir = dirname.last().expect("Failed to get last directory");
                try_cd(&PathBuf::from(".."))?;
                filter.clear();
                scroll_offset = 0;
                let files = get_file_names(".", show_hidden, sort_mode)?;
                index = 0;
                for (i, _) in files.iter().enumerate() {
                    if files[i].name == *old_dir {
                        index = i32::try_from(i).expect("Invalid index");
                        break;
                    }
                }
            }
            b'.' if !filter_mode => {
                show_hidden = !show_hidden;
                index = 0;
                scroll_offset = 0;
            }
            b's' if !filter_mode => {
                sort_mode = sort_mode.cycle();
                index = 0;
                scroll_offset = 0;
            }
            b'o' if !filter_mode => {
                let idx = usize::try_from(index.max(0)).expect("Invalid index");
                if idx < filtered_files.len() && !filtered_files[idx].path.is_dir() {
                    let path = filtered_files[idx].path.clone();
                    write!(stderr, "\x1b[?1049l")?;
                    stderr.flush()?;
                    drop(stderr);
                    let editor = std::env::var("EDITOR")
                        .unwrap_or_else(|_| String::from("xdg-open"));
                    let _ = std::process::Command::new(&editor).arg(&path).status();
                    stderr = io::stderr().into_raw_mode()?;
                    write!(stderr, "\x1b[?1049h{}{}{}", clear::All, cursor::Hide, cursor::Goto(1, 1))?;
                }
            }
            b'/' if !filter_mode => {
                filter_mode = true;
                filter.clear();
                index = 0;
                scroll_offset = 0;
            }
            0x1b if filter_mode => {
                filter_mode = false;
                filter.clear();
                index = 0;
                scroll_offset = 0;
            }
            0x0d if filter_mode => {
                filter_mode = false;
            }
            0x7f if filter_mode => {
                filter.pop();
                index = 0;
                scroll_offset = 0;
            }
            b if filter_mode && (0x20..=0x7e).contains(&b) => {
                filter.push(b as char);
                index = 0;
                scroll_offset = 0;
            }
            _ => {}
        }

        if index < 0 {
            index = 0;
        }

        let filtered_files = {
            let mut files = get_file_names(".", show_hidden, sort_mode)?;
            if !filter.is_empty() {
                let fl = filter.to_lowercase();
                files.retain(|f| f.name.to_lowercase().contains(&fl));
            }
            files
        };
        if index >= i32::try_from(filtered_files.len()).expect("Invalid index") {
            index = i32::try_from(filtered_files.len()).expect("Invalid index") - 1;
        }

        let (_, term_height) = terminal_size()?;
        let pane_height = term_height.saturating_sub(2) as i32;
        if index < scroll_offset {
            scroll_offset = index;
        }
        if index >= scroll_offset + pane_height {
            scroll_offset = index - pane_height + 1;
        }
        scroll_offset = scroll_offset.max(0);

        render(&mut stderr, index, show_hidden, &filter, filter_mode, sort_mode, scroll_offset)?;
    }

    write!(stderr, "{}{}\x1b[?1049l", cursor::Show, clear::All)?;
    stderr.flush()?;

    write_to_file("/tmp/directory-switcher", &get_cwd()?)?;
    Ok(())
}
