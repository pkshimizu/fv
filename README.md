# fv

[![CI](https://github.com/pkshimizu/fv/actions/workflows/ci.yml/badge.svg)](https://github.com/pkshimizu/fv/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/pkshimizu/fv)](https://github.com/pkshimizu/fv/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A fast, keyboard-driven TUI file manager that lives in your terminal.

`fv` is a Rust-based terminal file manager built on a Component Architecture. It keeps file
browsing, file operations, preview, search, and more on a single, clean screen — no GUI required.

![fv demo](https://raw.githubusercontent.com/pkshimizu/fv/main/site/images/hero-demo.gif)

**Landing page: <https://pkshimizu.github.io/fv/>** — features, screenshots, and details.

## Features

- **File operations** — copy, move, delete, rename, create files/directories, and zip/unzip.
  Long operations run as cancellable async jobs with progress.
- **Shell & commands** — launch a shell in the current directory or run an arbitrary command.
- **Preview** — text, rendered Markdown, images, and audio (with play/seek) in a side panel.
- **Search & view** — grep through the tree, incremental search, a list filter that hides non-matching files, directory jump, and a directory tree view.
- **File info & attributes** — inspect size, type, permissions, and timestamps.
- **Bookmarks** — save frequently used directories and jump to them quickly.
- **Contexts (tabs)** — keep several independent working directories and switch between them; the paste buffer is shared so you can copy in one and paste in another.
- **Yank** — copy selected paths to the system clipboard.

## Installation

### Homebrew (macOS Apple Silicon / Linux x86_64)

Since Homebrew 6.0.0, third-party taps must be trusted explicitly before their formulae load, so `brew trust` is required before installing.

```sh
brew tap pkshimizu/tap
brew trust --formula pkshimizu/tap/fv
brew install fv
```

### GitHub Releases

Download the archive for your platform from the
[releases page](https://github.com/pkshimizu/fv/releases), extract it, and place `fv` on your `PATH`.

```sh
# macOS (Apple Silicon)
tar xzf fv-aarch64-apple-darwin.tar.gz
# Linux (x86_64)
tar xzf fv-x86_64-unknown-linux-gnu.tar.gz

mv fv /usr/local/bin/
```

## Key bindings

Press `?` inside fv to open the help panel. The main key bindings in the file list:

### Navigation

| Key | Action |
| --- | --- |
| `Backspace` | Go to parent directory |
| `<` / `>` | Go back / forward in directory history |
| `~` | Go to home directory |
| `j` | Jump to directory |
| `g` | Grep in files |

### Selection & display

| Key | Action |
| --- | --- |
| `Space` | Toggle check mark |
| `Shift`+`A` | Select all / clear selection |
| `.` | Toggle dotfiles visibility |
| `s` | Sort files |
| `f` | Search files |
| `/` | Filter list (hide non-matches) |

### File operations

| Key | Action |
| --- | --- |
| `c` / `m` / `d` | Copy / move / delete files |
| `r` | Rename file |
| `k` / `n` | Create directory / file |
| `l` | Create a symlink pointing to the cursor file |
| `p` / `u` | Zip / unzip |
| `x` | Execute command |
| `y` | Yank paths to clipboard |
| `Ctrl`+`C` / `Ctrl`+`X` | Copy / cut to the paste buffer |
| `Ctrl`+`V` | Paste the buffer into the current directory |

### Panels & views

| Key | Action |
| --- | --- |
| `a` / `i` | Show file attributes / info |
| `t` / `v` | Show directory tree / preview file |
| `h` | Launch shell |
| `e` | Open in file manager |

### Bookmarks

| Key | Action |
| --- | --- |
| `b` | Show bookmarks |
| `+` / `-` | Add / remove bookmark |

### Contexts

| Key | Action |
| --- | --- |
| `Tab` / `Shift`+`Tab` | Switch to next / previous context |
| `w` | New context (duplicate current directory) |
| `Shift`+`W` | Close current context |

### App

| Key | Action |
| --- | --- |
| `o` | Settings |
| `?` | Show help |
| `q` | Quit |

## Environment variables

| Variable | Description |
| --- | --- |
| `FV_IMAGE_PROTOCOL` | Override the image preview protocol instead of auto-detecting it. Accepts `halfblocks`, `sixel`, `kitty`, or `iterm2` (case-insensitive). Unset or unrecognized values fall back to auto-detection. |

Some terminals (e.g. ttyd / xterm.js-based ones) report graphics protocols they cannot actually render, which leaves the image preview blank. In that case, set `FV_IMAGE_PROTOCOL=halfblocks` to force a protocol that renders everywhere:

```bash
FV_IMAGE_PROTOCOL=halfblocks fv
```

## Development

### Linux check with Docker

To build and run `fv` on Linux during development, use the provided Docker setup.
It ships a Rust stable toolchain plus the only system dependency Linux builds need
(`libasound2-dev`, for audio), bind-mounts your working tree, and gives the TUI a TTY.

```sh
# Build the image
docker compose build

# Run the TUI (default command is `cargo run`)
docker compose run --rm fv

# Build only / run the tests inside the container
docker compose run --rm fv cargo build
docker compose run --rm fv cargo test
```

Use `docker compose run --rm` (not `up`) so the container inherits your terminal's
TTY and the TUI renders and accepts keys. Press `q` to quit.

The container's `target/` and the cargo registry live in named volumes, so Linux
build artifacts never collide with the host's (e.g. macOS) `target/`, and rebuilds
stay fast across runs.

## License

Licensed under the [MIT License](LICENSE).
