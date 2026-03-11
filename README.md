![Banner](https://s-christy.com/sbs/status-banner.svg?icon=file/folder_open&hue=180&title=Directory%20Switcher&description=A%20keyboard-driven%20terminal%20directory%20navigator)

## Overview

Directory Switcher is a keyboard-driven terminal file navigator written in
Rust. It presents a three-pane view (parent directory, current directory, and a
preview of the selected item) allowing fast, intuitive navigation through the
filesystem without leaving the terminal.

When you quit the tool, it writes the current directory to a temporary file and
a shell function (`ds`) reads it back to `cd` into the selected location. This
makes it a practical replacement for `cd` when you need to explore before
committing to a destination.

The tool supports filtering entries by typing, sorting by name, size, or
modification time, bookmarking frequently visited directories, and opening
files directly in `$EDITOR`.

## Features

## Keybindings

## Installation

## Configuration

## Dependencies

## License

This work is licensed under the GNU General Public License version 3 (GPLv3).

[<img src="https://s-christy.com/status-banner-service/GPLv3_Logo.svg" width="150" />](https://www.gnu.org/licenses/gpl-3.0.en.html)
