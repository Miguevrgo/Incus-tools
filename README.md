# Incus-tools

A keyboard-driven Terminal UI (TUI) for managing [Incus](https://linuxcontainers.org/incus/) containers and VMs. Includes two implementations: a full-featured Rust version built with **ratatui** and a Python prototype using **curses**.

## Features

- **Create** containers from 8 Linux distributions (Ubuntu, Debian, CentOS, Fedora, AlmaLinux, Rocky, Amazon Linux, openSUSE) with an interactive OS/version grid selector.
- **List** all instances in a scrollable table showing name, OS, release, state, and IPv4 address.
- **Enter** a running instance shell directly from the TUI.
- **Stop** running instances.
- **Delete** individual instances or all at once with a confirmation prompt.
- Automatic package-manager cache refresh on newly created containers (`apt`, `dnf`, `yum`, `zypper`).
- Responsive layout that adapts from wide single-row grids down to narrow single-column mode.
- Vim-style navigation (`h`/`j`/`k`/`l`) alongside arrow keys.
- Nerd Font icons throughout the interface.

## Requirements

- [Incus](https://linuxcontainers.org/incus/) installed and running.
- Current user in the `incus-admin` group (or equivalent permissions on the Incus socket).
- A [Nerd Font](https://www.nerdfonts.com/) terminal font for icon rendering.

### Rust version

- Rust toolchain (1.70+)

### Python version

- Python 3.10+ (uses `curses`, included in the standard library on Linux/macOS)

## Installation

### Rust (recommended)

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release
./target/release/manager
```

### Rust for Wazuh
```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release --features wazuh
./target/release/manager
```
### Python

```bash
python3 manager.py
```

## Usage

Navigate menus with arrow keys or `h`/`j`/`k`/`l`. Press `Enter` to select, `q` or `Esc` to go back.

| Action | Description |
|--------|-------------|
| Create | Pick a distro and version, name the instance, and launch it. Drops you into a shell once ready. |
| List   | View all instances in a table with state and IP info. |
| Enter  | Open a bash shell inside a running instance. |
| Stop   | Stop a running instance. |
| Delete | Remove an instance (force-delete). Supports bulk deletion. |

## License

[MIT](LICENSE)
