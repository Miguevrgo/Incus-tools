use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};
use std::io::{self, Write, stdout};
use std::process::Command;
use std::time::Duration;

// ── Nerd-font icons ──────────────────────────────────────────────────────────
const ICON_VM: &str = "\u{f0a0}";
const ICON_CREATE: &str = "\u{f055}";
const ICON_LIST: &str = "\u{f0ca}";
const ICON_ENTER: &str = "\u{f489}";
const ICON_STOP: &str = "\u{f28d}";
const ICON_DELETE: &str = "\u{f1f8}";
const ICON_EXIT: &str = "\u{f08b}";
const ICON_OS: &str = "\u{f17c}";
const ICON_INSTANCE: &str = "\u{f2d0}";
const ICON_ERROR: &str = "\u{f057}";
const ICON_RUNNING: &str = "\u{f444}";
const ICON_STOPPED: &str = "\u{f04d}";
const ICON_EMPTY: &str = "\u{f49e}";

// ── OS grid data ─────────────────────────────────────────────────────────────
struct OsEntry {
    name: &'static str,
    versions: &'static [&'static str],
    aliases: &'static [&'static str],
}

const OS_GRID: &[OsEntry] = &[
    OsEntry {
        name: "Ubuntu \u{f31b}",
        versions: &[
            "25.10 (questing)",
            "25.04 (plucky)",
            "24.04 (noble)",
            "22.04 (jammy)",
        ],
        aliases: &[
            "images:ubuntu/questing",
            "images:ubuntu/plucky",
            "images:ubuntu/noble",
            "images:ubuntu/jammy",
        ],
    },
    OsEntry {
        name: "Debian \u{f306}",
        versions: &[
            "14 (forky)",
            "13 (trixie)",
            "12 (bookworm)",
            "11 (bullseye)",
        ],
        aliases: &[
            "images:debian/14",
            "images:debian/13",
            "images:debian/12",
            "images:debian/11",
        ],
    },
    OsEntry {
        name: "CentOS \u{f304}",
        versions: &["10-Stream", "9-Stream"],
        aliases: &["images:centos/10-Stream", "images:centos/9-Stream"],
    },
    OsEntry {
        name: "Fedora \u{f30a}",
        versions: &["43", "42", "41"],
        aliases: &["images:fedora/43", "images:fedora/42", "images:fedora/41"],
    },
    OsEntry {
        name: "AlmaLinux \u{f31e}",
        versions: &["10 (RHEL-compat)", "9 (RHEL-compat)", "8 (RHEL-compat)"],
        aliases: &[
            "images:almalinux/10",
            "images:almalinux/9",
            "images:almalinux/8",
        ],
    },
    OsEntry {
        name: "Rocky \u{f31e}",
        versions: &["10", "9", "8"],
        aliases: &[
            "images:rockylinux/10",
            "images:rockylinux/9",
            "images:rockylinux/8",
        ],
    },
    OsEntry {
        name: "Amazon \u{f270}",
        versions: &["2023"],
        aliases: &["images:amazonlinux/2023"],
    },
    OsEntry {
        name: "openSUSE \u{f314}",
        versions: &["16", "15"],
        aliases: &["images:opensuse/16.0", "images:opensuse/15.6"],
    },
];

// ── Main menu ────────────────────────────────────────────────────────────────
const MAIN_MENU: &[(&str, &str)] = &[
    ("Create", ICON_CREATE),
    ("List", ICON_LIST),
    ("Enter", ICON_ENTER),
    ("Stop", ICON_STOP),
    ("Delete", ICON_DELETE),
    ("Exit", ICON_EXIT),
];

// ── Incus helpers ────────────────────────────────────────────────────────────
fn check_incus_available() -> bool {
    which::which("incus").is_ok()
}

fn check_incus_permissions() -> Option<String> {
    let result = Command::new("incus").arg("list").output();
    match result {
        Ok(output) => {
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                if stderr.to_lowercase().contains("permission")
                    || stderr.to_lowercase().contains("unix.socket")
                {
                    let user = Command::new("whoami")
                        .output()
                        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                        .unwrap_or_else(|_| "???".into());
                    Some(format!(
                        "Permission denied on the incus socket.\n  Fix: sudo usermod -aG incus-admin {} && newgrp incus-admin",
                        user
                    ))
                } else {
                    Some(format!("incus error: {}", stderr))
                }
            } else {
                None
            }
        }
        Err(e) => Some(e.to_string()),
    }
}

#[derive(Clone)]
struct Instance {
    name: String,
    os: String,
    release: String,
    state: String,
    ipv4: String,
}

fn get_incus_instances() -> Vec<Instance> {
    if !check_incus_available() {
        return vec![];
    }
    let result = Command::new("incus")
        .args([
            "list",
            "-c",
            "n,config:image.os,config:image.release,s,4",
            "--format",
            "csv",
        ])
        .output();

    match result {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout);
            text.lines()
                .filter(|l| !l.trim().is_empty())
                .map(|line| {
                    let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
                    let ipv4 = parts
                        .get(4)
                        .unwrap_or(&"")
                        .split_whitespace()
                        .next()
                        .unwrap_or("");
                    Instance {
                        name: parts.first().unwrap_or(&"").to_string(),
                        os: parts.get(1).unwrap_or(&"").to_string(),
                        release: parts.get(2).unwrap_or(&"").to_string(),
                        state: parts.get(3).unwrap_or(&"").to_string(),
                        ipv4: ipv4.to_string(),
                    }
                })
                .collect()
        }
        Err(_) => vec![],
    }
}

// ── TUI state machine ────────────────────────────────────────────────────────
enum Screen {
    MainMenu,
    GridSelect,
    NameInput {
        distro_idx: usize,
        version_idx: usize,
        input: String,
    },
    InstanceTable {
        instances: Vec<Instance>,
    },
    InstanceSelect {
        action: String,
        instances: Vec<Instance>,
    },
    Confirm {
        message: String,
        selected: usize, // 0 = Yes, 1 = No
        pending_action: PendingAction,
    },
    Quit,
}

#[derive(Clone)]
enum PendingAction {
    DeleteAll(Vec<String>),
    #[allow(dead_code)]
    None,
}

struct App {
    screen: Screen,
    main_idx: usize,
    grid_col: usize,
    grid_row: usize,
    list_idx: usize,
    table_offset: usize,
}

impl App {
    fn new() -> Self {
        Self {
            screen: Screen::MainMenu,
            main_idx: 0,
            grid_col: 0,
            grid_row: 0,
            list_idx: 0,
            table_offset: 0,
        }
    }
}

// ── Restore / enter terminal ─────────────────────────────────────────────────
fn leave_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
}

fn enter_tui(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) {
    let _ = enable_raw_mode();
    let _ = execute!(terminal.backend_mut(), EnterAlternateScreen);
    let _ = terminal.hide_cursor();
    let _ = terminal.clear();
}

fn run_cli_commands(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    commands: &[&[&str]],
    pause: bool,
) {
    leave_tui(terminal);
    let _ = Command::new("clear").status();
    for cmd in commands {
        if cmd.is_empty() {
            continue;
        }
        let _ = Command::new(cmd[0]).args(&cmd[1..]).status();
    }
    if pause {
        print!("\nPress Enter to continue...");
        let _ = io::stdout().flush();
        let mut buf = String::new();
        let _ = io::stdin().read_line(&mut buf);
    }
    enter_tui(terminal);
}

// ── Drawing helpers ──────────────────────────────────────────────────────────
fn help_line(text: &str) -> Paragraph<'_> {
    Paragraph::new(Line::from(Span::styled(
        text.to_string(),
        Style::default().fg(Color::Blue),
    )))
}

fn draw_main_menu(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Clear, area);

    let title = format!("{}  VM Manager", ICON_VM);
    let title_p = Paragraph::new(Line::from(Span::styled(
        title,
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    if area.height > 2 {
        f.render_widget(title_p, Rect::new(2, 1, area.width.saturating_sub(4), 1));
    }

    for (idx, (label, icon)) in MAIN_MENU.iter().enumerate() {
        let y = 4 + (idx as u16) * 2;
        if y >= area.height.saturating_sub(2) {
            break;
        }
        let text = format!("{}  {}", icon, label);
        let (style, prefix) = if idx == app.main_idx {
            (
                Style::default()
                    .bg(Color::White)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
                "  ",
            )
        } else {
            (Style::default(), "   ")
        };
        let line = Line::from(Span::styled(format!("{}{}", prefix, text), style));
        f.render_widget(
            Paragraph::new(line),
            Rect::new(4, y, area.width.saturating_sub(8), 1),
        );
    }

    let help = "\u{f062}/\u{f063} (k/j) navigate  \u{2502}  Enter select  \u{2502}  q quit";
    if area.height > 2 {
        f.render_widget(
            help_line(help),
            Rect::new(2, area.height - 2, area.width.saturating_sub(4), 1),
        );
    }
}

fn compute_box_width() -> u16 {
    let mut box_w: u16 = 0;
    for entry in OS_GRID {
        let hdr_w = entry.name.chars().count() as u16;
        let ver_w = entry
            .versions
            .iter()
            .map(|v| v.chars().count() as u16 + 4)
            .max()
            .unwrap_or(0);
        let w = hdr_w.max(ver_w) + 4;
        if w > box_w {
            box_w = w;
        }
    }
    box_w
}

fn draw_grid(f: &mut Frame, app: &App) {
    let area = f.area();
    f.render_widget(Clear, area);

    let title = format!("{}  Select OS and Version", ICON_OS);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))),
        Rect::new(2, 1, area.width.saturating_sub(4), 1),
    );

    let num_cols = OS_GRID.len();
    let max_versions = OS_GRID.iter().map(|e| e.versions.len()).max().unwrap_or(0);
    let box_w = compute_box_width();
    let box_h = max_versions as u16 + 2;
    let usable = area.width.saturating_sub(4);
    let top_count = num_cols.div_ceil(2);

    enum GridLayout {
        SingleRow,
        TwoRow,
        Narrow,
    }
    let layout = if usable >= (num_cols as u16) * box_w {
        GridLayout::SingleRow
    } else if usable >= (top_count as u16) * box_w {
        GridLayout::TwoRow
    } else {
        GridLayout::Narrow
    };

    let draw_column = |f: &mut Frame, c_idx: usize, x_pos: u16, base_y: u16, bw: u16, bh: u16| {
        let entry = &OS_GRID[c_idx];
        let is_active = c_idx == app.grid_col;
        let border_color = if is_active {
            Color::Green
        } else {
            Color::White
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                format!(" {} ", entry.name),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
        f.render_widget(block, Rect::new(x_pos, base_y, bw, bh));

        for (r_idx, ver) in entry.versions.iter().enumerate() {
            let y = base_y + 1 + r_idx as u16;
            if y >= area.height.saturating_sub(2) {
                break;
            }
            let (style, prefix) = if c_idx == app.grid_col && r_idx == app.grid_row {
                (
                    Style::default()
                        .bg(Color::White)
                        .fg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                    "\u{25b6} ",
                )
            } else {
                (Style::default(), "  ")
            };
            let text = format!("{}{}", prefix, ver);
            let max_w = bw.saturating_sub(4) as usize;
            let truncated: String = text.chars().take(max_w).collect();
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(truncated, style))),
                Rect::new(x_pos + 2, y, bw.saturating_sub(4), 1),
            );
        }
    };

    match layout {
        GridLayout::SingleRow => {
            let extra = usable.saturating_sub((num_cols as u16) * box_w);
            let gap = extra / (num_cols as u16).max(1);
            let mut x_pos: u16 = 2 + gap / 2;
            for c_idx in 0..num_cols {
                draw_column(f, c_idx, x_pos, 3, box_w, box_h);
                x_pos += box_w + gap;
            }
        }
        GridLayout::TwoRow => {
            let extra = usable.saturating_sub((top_count as u16) * box_w);
            let gap = extra / (top_count as u16).max(1);
            let mut x_pos: u16 = 2 + gap / 2;
            for c_idx in 0..top_count {
                draw_column(f, c_idx, x_pos, 3, box_w, box_h);
                x_pos += box_w + gap;
            }
            let bot_start_y = 3 + box_h + 1;
            let mut x_pos: u16 = 2 + gap / 2;
            for c_idx_bot in 0..(num_cols - top_count) {
                let global_idx = top_count + c_idx_bot;
                draw_column(f, global_idx, x_pos, bot_start_y, box_w, box_h);
                x_pos += box_w + gap;
            }
        }
        GridLayout::Narrow => {
            let entry = &OS_GRID[app.grid_col];
            let title_str = format!(" {}  ({}/{}) ", entry.name, app.grid_col + 1, num_cols);
            let w = area.width.saturating_sub(4);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(Span::styled(
                    title_str,
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ));
            f.render_widget(block, Rect::new(2, 3, w, box_h));

            for (r_idx, ver) in entry.versions.iter().enumerate() {
                let y = 4 + r_idx as u16;
                let (style, prefix) = if r_idx == app.grid_row {
                    (
                        Style::default()
                            .bg(Color::White)
                            .fg(Color::Black)
                            .add_modifier(Modifier::BOLD),
                        "\u{25b6} ",
                    )
                } else {
                    (Style::default(), "  ")
                };
                let text = format!("{}{}", prefix, ver);
                let max_w = w.saturating_sub(4) as usize;
                let truncated: String = text.chars().take(max_w).collect();
                f.render_widget(
                    Paragraph::new(Line::from(Span::styled(truncated, style))),
                    Rect::new(4, y, w.saturating_sub(4), 1),
                );
            }
        }
    }

    let help = "\u{f060}/\u{f061} h/l: distro  \u{2502}  \u{f062}/\u{f063} k/j: version  \u{2502}  Enter: select  \u{2502}  q: back";
    if area.height > 2 {
        f.render_widget(
            help_line(help),
            Rect::new(2, area.height - 2, area.width.saturating_sub(4), 1),
        );
    }
}

fn draw_name_input(f: &mut Frame, distro_idx: usize, version_idx: usize, input: &str) {
    let area = f.area();
    f.render_widget(Clear, area);

    let entry = &OS_GRID[distro_idx];
    let ver = entry.versions[version_idx];
    let title = format!("{}  Creating {} {}", ICON_CREATE, entry.name, ver);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))),
        Rect::new(2, 2, area.width.saturating_sub(4), 1),
    );

    let prompt = format!("{}  Instance name: {}\u{2588}", ICON_INSTANCE, input);
    f.render_widget(
        Paragraph::new(Line::from(Span::raw(prompt))),
        Rect::new(2, 4, area.width.saturating_sub(4), 1),
    );

    let help = "Type name and press Enter  \u{2502}  Esc: cancel";
    if area.height > 2 {
        f.render_widget(
            help_line(help),
            Rect::new(2, area.height - 2, area.width.saturating_sub(4), 1),
        );
    }
}

fn draw_instance_table(f: &mut Frame, instances: &[Instance], offset: usize) {
    let area = f.area();
    f.render_widget(Clear, area);

    let title = format!("{}  Instances", ICON_LIST);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))),
        Rect::new(2, 1, area.width.saturating_sub(4), 1),
    );

    if instances.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("{}  No instances found.", ICON_EMPTY),
                Style::default().fg(Color::Red),
            ))),
            Rect::new(4, 3, area.width.saturating_sub(8), 1),
        );
    } else {
        let headers = ["NAME", "OS", "RELEASE", "STATE", "IPv4"];
        let mut col_w: Vec<usize> = (0..5)
            .map(|i| {
                let data_max = instances
                    .iter()
                    .map(|inst| match i {
                        0 => inst.name.len(),
                        1 => inst.os.len(),
                        2 => inst.release.len(),
                        3 => inst.state.len(),
                        4 => inst.ipv4.len(),
                        _ => 0,
                    })
                    .max()
                    .unwrap_or(0);
                headers[i].len().max(data_max)
            })
            .collect();

        let usable = area.width.saturating_sub(4) as usize;
        let total: usize = col_w.iter().sum::<usize>() + 16;
        if total > usable {
            let excess = total - usable;
            col_w[0] = col_w[0].saturating_sub(excess / 2).max(6);
            col_w[4] = col_w[4].saturating_sub(excess - excess / 2).max(7);
        }

        let sep_len: usize = col_w.iter().sum::<usize>() + 16;
        let sep: String = "\u{2500}".repeat(sep_len.min(usable));
        let sep_style = Style::default().fg(Color::Blue);

        f.render_widget(
            Paragraph::new(Line::from(Span::styled(sep.clone(), sep_style))),
            Rect::new(2, 3, area.width.saturating_sub(4), 1),
        );

        // headers
        let mut spans = Vec::new();
        for (i, hdr) in headers.iter().enumerate() {
            let padded = format!("{:<width$}", hdr, width = col_w[i]);
            spans.push(Span::styled(
                padded,
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ));
            if i < 4 {
                spans.push(Span::raw("    "));
            }
        }
        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(2, 4, area.width.saturating_sub(4), 1),
        );

        f.render_widget(
            Paragraph::new(Line::from(Span::styled(sep.clone(), sep_style))),
            Rect::new(2, 5, area.width.saturating_sub(4), 1),
        );

        let list_top: u16 = 6;
        let list_height = area.height.saturating_sub(list_top + 3) as usize;
        let visible = &instances[offset..instances.len().min(offset + list_height)];

        for (idx, inst) in visible.iter().enumerate() {
            let y = list_top + idx as u16;
            let status_icon = if inst.state == "RUNNING" {
                ICON_RUNNING
            } else {
                ICON_STOPPED
            };
            let status_color = if inst.state == "RUNNING" {
                Color::Green
            } else {
                Color::Red
            };

            let fields = [&inst.name, &inst.os, &inst.release, &inst.state, &inst.ipv4];
            let mut spans = Vec::new();
            for (i, cell) in fields.iter().enumerate() {
                let truncated: String = cell.chars().take(col_w[i]).collect();
                let padded = format!("{:<width$}", truncated, width = col_w[i]);
                if i == 3 {
                    spans.push(Span::styled(
                        format!("{} ", status_icon),
                        Style::default().fg(status_color),
                    ));
                    spans.push(Span::styled(padded, Style::default().fg(status_color)));
                } else {
                    spans.push(Span::raw(padded));
                }
                if i < 4 {
                    spans.push(Span::raw("    "));
                }
            }
            f.render_widget(
                Paragraph::new(Line::from(spans)),
                Rect::new(2, y, area.width.saturating_sub(4), 1),
            );
        }

        let bottom_sep_y = list_top + visible.len() as u16;
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(sep, sep_style))),
            Rect::new(2, bottom_sep_y, area.width.saturating_sub(4), 1),
        );

        if instances.len() > list_height {
            let hint = format!(
                " {}-{}/{} ",
                offset + 1,
                (offset + list_height).min(instances.len()),
                instances.len()
            );
            let hint_x = area.width.saturating_sub(hint.len() as u16 + 2);
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(hint, sep_style))),
                Rect::new(hint_x, bottom_sep_y, area.width.saturating_sub(hint_x), 1),
            );
        }
    }

    let help = "\u{f062}/\u{f063} k/j: scroll  \u{2502}  q: back";
    if area.height > 2 {
        f.render_widget(
            help_line(help),
            Rect::new(2, area.height - 2, area.width.saturating_sub(4), 1),
        );
    }
}

fn draw_list_menu(
    f: &mut Frame,
    title: &str,
    instances: &[Instance],
    current: usize,
    delete_mode: bool,
) {
    let area = f.area();
    f.render_widget(Clear, area);

    let title_str = format!("{}  {}", ICON_LIST, title);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            title_str,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))),
        Rect::new(2, 1, area.width.saturating_sub(4), 1),
    );

    let mut y_offset: u16 = 4;

    if delete_mode {
        let label = format!("{}  Delete All", ICON_DELETE);
        let (style, prefix) = if current == 0 {
            (
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                "\u{25b6} ",
            )
        } else {
            (Style::default().fg(Color::Red), "  ")
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("{}{}", prefix, label),
                style,
            ))),
            Rect::new(4, y_offset, area.width.saturating_sub(8), 1),
        );
        y_offset += 1;
    }

    let row_offset: usize = if delete_mode { 1 } else { 0 };

    for (idx, inst) in instances.iter().enumerate() {
        let visual_idx = idx + row_offset;
        let y = y_offset + idx as u16;
        if y >= area.height.saturating_sub(2) {
            break;
        }

        let status_icon = if inst.state == "RUNNING" {
            ICON_RUNNING
        } else {
            ICON_STOPPED
        };
        let status_color = if inst.state == "RUNNING" {
            Color::Green
        } else {
            Color::Red
        };

        if visual_idx == current {
            let style = Style::default()
                .bg(Color::White)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD);
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("\u{25b6} ", style),
                    Span::styled(format!("{}  ", status_icon), style),
                    Span::styled(inst.name.clone(), style),
                ])),
                Rect::new(4, y, area.width.saturating_sub(8), 1),
            );
        } else {
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        format!("{}  ", status_icon),
                        Style::default().fg(status_color),
                    ),
                    Span::raw(inst.name.clone()),
                ])),
                Rect::new(4, y, area.width.saturating_sub(8), 1),
            );
        }
    }

    let help = "\u{f062}/\u{f063} k/j: navigate  \u{2502}  Enter: select  \u{2502}  q/h: back";
    if area.height > 2 {
        f.render_widget(
            help_line(help),
            Rect::new(2, area.height - 2, area.width.saturating_sub(4), 1),
        );
    }
}

fn draw_confirm(f: &mut Frame, message: &str, selected: usize) {
    let area = f.area();
    f.render_widget(Clear, area);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("{}  {}", ICON_ERROR, message),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ))),
        Rect::new(2, 2, area.width.saturating_sub(4), 1),
    );

    let options = ["Yes", "No"];
    for (idx, label) in options.iter().enumerate() {
        let y = 5 + idx as u16;
        let (style, prefix) = if idx == selected {
            (
                Style::default()
                    .bg(Color::White)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD),
                "  \u{25b6} ",
            )
        } else {
            (Style::default(), "    ")
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!("{}{}", prefix, label),
                style,
            ))),
            Rect::new(4, y, area.width.saturating_sub(8), 1),
        );
    }

    let help = "\u{f062}/\u{f063} k/j: navigate  \u{2502}  Enter: confirm  \u{2502}  q: cancel";
    if area.height > 2 {
        f.render_widget(
            help_line(help),
            Rect::new(2, area.height - 2, area.width.saturating_sub(4), 1),
        );
    }
}

// ── Main ─────────────────────────────────────────────────────────────────────
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Pre-flight checks
    if !check_incus_available() {
        eprintln!(
            "\n{}  Error: incus command not found. Please install LXD/Incus first.",
            ICON_ERROR
        );
        eprint!("Press Enter to continue...");
        io::stderr().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        return Ok(());
    }
    if let Some(err) = check_incus_permissions() {
        eprintln!("\n{}  Error: {}", ICON_ERROR, err);
        eprint!("Press Enter to continue...");
        io::stderr().flush()?;
        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
        return Ok(());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let mut app = App::new();

    loop {
        // Draw
        terminal.draw(|f| match &app.screen {
            Screen::MainMenu => draw_main_menu(f, &app),
            Screen::GridSelect => draw_grid(f, &app),
            Screen::NameInput {
                distro_idx,
                version_idx,
                input,
            } => draw_name_input(f, *distro_idx, *version_idx, input),
            Screen::InstanceTable { instances } => {
                draw_instance_table(f, instances, app.table_offset);
            }
            Screen::InstanceSelect { action, instances } => {
                let title = format!("Select instance to {}:", action);
                let delete_mode = action == "Delete";
                draw_list_menu(f, &title, instances, app.list_idx, delete_mode);
            }
            Screen::Confirm {
                message, selected, ..
            } => draw_confirm(f, message, *selected),
            Screen::Quit => {}
        })?;

        if matches!(app.screen, Screen::Quit) {
            break;
        }

        // Handle input
        if event::poll(Duration::from_millis(300))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match &mut app.screen {
                Screen::MainMenu => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.screen = Screen::Quit;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if app.main_idx > 0 {
                            app.main_idx -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if app.main_idx < MAIN_MENU.len() - 1 {
                            app.main_idx += 1;
                        }
                    }
                    KeyCode::Enter => {
                        let action = MAIN_MENU[app.main_idx].0;
                        match action {
                            "Exit" => {
                                app.screen = Screen::Quit;
                            }
                            "Create" => {
                                if !check_incus_available() {
                                    leave_tui(&mut terminal);
                                    eprintln!("\n{}  Error: incus not found.", ICON_ERROR);
                                    print!("Press Enter to continue...");
                                    let _ = io::stdout().flush();
                                    let mut buf = String::new();
                                    let _ = io::stdin().read_line(&mut buf);
                                    enter_tui(&mut terminal);
                                } else {
                                    app.grid_col = 0;
                                    app.grid_row = 0;
                                    app.screen = Screen::GridSelect;
                                }
                            }
                            "List" => {
                                app.table_offset = 0;
                                let instances = get_incus_instances();
                                app.screen = Screen::InstanceTable { instances };
                            }
                            _ => {
                                // Enter, Stop, Delete
                                let instances = get_incus_instances();
                                if instances.is_empty() {
                                    run_cli_commands(
                                        &mut terminal,
                                        &[&[
                                            "echo",
                                            &format!("{}  No instances found.", ICON_EMPTY),
                                        ]],
                                        true,
                                    );
                                } else {
                                    app.list_idx = 0;
                                    app.screen = Screen::InstanceSelect {
                                        action: action.to_string(),
                                        instances,
                                    };
                                }
                            }
                        }
                    }
                    _ => {}
                },

                Screen::GridSelect => {
                    let num_cols = OS_GRID.len();
                    let top_count = num_cols.div_ceil(2);
                    let usable = terminal.size()?.width.saturating_sub(4);
                    let box_w = compute_box_width();
                    let is_two_row =
                        usable < (num_cols as u16) * box_w && usable >= (top_count as u16) * box_w;

                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app.screen = Screen::MainMenu;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.grid_row > 0 {
                                app.grid_row -= 1;
                            } else if is_two_row && app.grid_col >= top_count {
                                app.grid_col -= top_count;
                                let max_r = OS_GRID[app.grid_col].versions.len().saturating_sub(1);
                                app.grid_row = max_r;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            let max_row = OS_GRID[app.grid_col].versions.len().saturating_sub(1);
                            if app.grid_row < max_row {
                                app.grid_row += 1;
                            } else if is_two_row && app.grid_col < top_count {
                                let next_col = app.grid_col + top_count;
                                if next_col < num_cols {
                                    app.grid_col = next_col;
                                    app.grid_row = 0;
                                }
                            }
                        }
                        KeyCode::Left | KeyCode::Char('h') => {
                            if app.grid_col > 0 {
                                app.grid_col -= 1;
                                let max_r = OS_GRID[app.grid_col].versions.len().saturating_sub(1);
                                if app.grid_row > max_r {
                                    app.grid_row = max_r;
                                }
                            }
                        }
                        KeyCode::Right | KeyCode::Char('l') => {
                            if app.grid_col < num_cols - 1 {
                                app.grid_col += 1;
                                let max_r = OS_GRID[app.grid_col].versions.len().saturating_sub(1);
                                if app.grid_row > max_r {
                                    app.grid_row = max_r;
                                }
                            }
                        }
                        KeyCode::Enter => {
                            let di = app.grid_col;
                            let vi = app.grid_row;
                            app.screen = Screen::NameInput {
                                distro_idx: di,
                                version_idx: vi,
                                input: String::new(),
                            };
                        }
                        _ => {}
                    }
                }

                Screen::NameInput {
                    distro_idx,
                    version_idx,
                    input,
                } => match key.code {
                    KeyCode::Esc => {
                        app.screen = Screen::GridSelect;
                    }
                    KeyCode::Backspace => {
                        input.pop();
                    }
                    KeyCode::Char(c) => {
                        if input.len() < 50 {
                            input.push(c);
                        }
                    }
                    KeyCode::Enter => {
                        let name = input.trim().to_string();
                        if name.is_empty() {
                            app.screen = Screen::GridSelect;
                        } else {
                            let di = *distro_idx;
                            let vi = *version_idx;
                            let entry = &OS_GRID[di];
                            let image = entry.aliases[vi];
                            let distro_name = entry.name;

                            leave_tui(&mut terminal);
                            let _ = Command::new("clear").status();

                            // 1. Launch
                            let _ = Command::new("incus")
                                .args(["launch", image, &name])
                                .status();

                            // 2. Wait for network
                            let wait_cmd = "for i in $(seq 1 30); do ping -c 1 -W 1 1.1.1.1 >/dev/null 2>&1 && break || sleep 1; done";
                            let _ = Command::new("incus")
                                .args(["exec", &name, "--", "sh", "-c", wait_cmd])
                                .status();

                            // 3. Package manager update
                            if distro_name.contains("Ubuntu") || distro_name.contains("Debian") {
                                let _ = Command::new("incus")
                                    .args([
                                        "exec",
                                        &name,
                                        "--",
                                        "env",
                                        "DEBIAN_FRONTEND=noninteractive",
                                        "apt-get",
                                        "update",
                                        "-y",
                                    ])
                                    .status();
                            } else if distro_name.contains("CentOS")
                                || distro_name.contains("Fedora")
                                || distro_name.contains("AlmaLinux")
                                || distro_name.contains("Rocky")
                                || distro_name.contains("Amazon")
                            {
                                let _ = Command::new("incus")
                                    .args(["exec", &name, "--", "dnf", "makecache"])
                                    .status();
                            } else if distro_name.contains("openSUSE") {
                                let _ = Command::new("incus")
                                    .args(["exec", &name, "--", "zypper", "refresh"])
                                    .status();
                            }

                            // 4. Wazuh build deps + MOTD (only with --features wazuh)
                            #[cfg(feature = "wazuh")]
                            {
                                // Echo git clone instruction
                                let _ = Command::new("incus")
                                    .args([
                                        "exec",
                                        &name,
                                        "--",
                                        "echo",
                                        "git clone https://github.com/wazuh/wazuh.git",
                                    ])
                                    .status();

                                // Install build dependencies
                                if distro_name.contains("Ubuntu") || distro_name.contains("Debian")
                                {
                                    let _ = Command::new("incus")
                                        .args([
                                            "exec",
                                            &name,
                                            "--",
                                            "env",
                                            "DEBIAN_FRONTEND=noninteractive",
                                            "apt-get",
                                            "install",
                                            "-y",
                                            "python3",
                                            "gcc",
                                            "g++",
                                            "make",
                                            "libc6-dev",
                                            "curl",
                                            "policycoreutils",
                                            "automake",
                                            "autoconf",
                                            "libtool",
                                            "libssl-dev",
                                            "procps",
                                            "build-essential",
                                            "cmake",
                                            "git",
                                        ])
                                        .status();
                                } else if distro_name.contains("CentOS")
                                    || distro_name.contains("Fedora")
                                    || distro_name.contains("AlmaLinux")
                                    || distro_name.contains("Rocky")
                                    || distro_name.contains("Amazon")
                                {
                                    let _ = Command::new("incus")
                                        .args([
                                            "exec",
                                            &name,
                                            "--",
                                            "dnf",
                                            "install",
                                            "-y",
                                            "python3",
                                            "gcc",
                                            "gcc-c++",
                                            "make",
                                            "glibc-devel",
                                            "curl",
                                            "policycoreutils",
                                            "automake",
                                            "autoconf",
                                            "libtool",
                                            "openssl-devel",
                                            "procps-ng",
                                            "cmake",
                                            "git",
                                        ])
                                        .status();
                                } else if distro_name.contains("openSUSE") {
                                    let _ = Command::new("incus")
                                        .args([
                                            "exec",
                                            &name,
                                            "--",
                                            "zypper",
                                            "install",
                                            "-y",
                                            "python3",
                                            "gcc",
                                            "gcc-c++",
                                            "make",
                                            "glibc-devel",
                                            "curl",
                                            "policycoreutils",
                                            "automake",
                                            "autoconf",
                                            "libtool",
                                            "libopenssl-devel",
                                            "procps",
                                            "cmake",
                                            "git",
                                        ])
                                        .status();
                                }

                                // Write /etc/motd with Wazuh quickstart
                                let motd = concat!(
                                    "╔══════════════════════════════════════════════════════════════╗\n",
                                    "║                  Wazuh Development Container                ║\n",
                                    "╠══════════════════════════════════════════════════════════════╣\n",
                                    "║  Build dependencies are pre-installed.                      ║\n",
                                    "║                                                             ║\n",
                                    "║  Quick install (all-in-one):                                ║\n",
                                    "║    curl -sO https://packages.wazuh.com/4.14/wazuh-install.sh║\n",
                                    "║    sudo bash ./wazuh-install.sh -a                          ║\n",
                                    "║                                                             ║\n",
                                    "║  Docs: https://documentation.wazuh.com                      ║\n",
                                    "╚══════════════════════════════════════════════════════════════╝\n",
                                );
                                let motd_cmd = format!(
                                    "printf '{}' > /etc/motd",
                                    motd.replace('\'', "'\\''")
                                );
                                let _ = Command::new("incus")
                                    .args(["exec", &name, "--", "sh", "-c", &motd_cmd])
                                    .status();

                                // Echo quickstart commands
                                let _ = Command::new("incus")
                                    .args([
                                        "exec",
                                        &name,
                                        "--",
                                        "echo",
                                        "curl -sO https://packages.wazuh.com/4.14/wazuh-install.sh && sudo bash ./wazuh-install.sh -a",
                                    ])
                                    .status();
                            }

                            // 5. Enter shell
                            let _ = Command::new("incus")
                                .args(["exec", &name, "--", "bash"])
                                .status();

                            enter_tui(&mut terminal);
                            app.screen = Screen::MainMenu;
                        }
                    }
                    _ => {}
                },

                Screen::InstanceTable { instances } => {
                    let list_height = terminal.size()?.height.saturating_sub(9) as usize;
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('h') | KeyCode::Esc | KeyCode::Left => {
                            app.screen = Screen::MainMenu;
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if list_height > 0 && app.table_offset + list_height < instances.len() {
                                app.table_offset += 1;
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.table_offset > 0 {
                                app.table_offset -= 1;
                            }
                        }
                        _ => {}
                    }
                }

                Screen::InstanceSelect { action, instances } => {
                    let is_delete = action == "Delete";
                    let row_offset: usize = if is_delete { 1 } else { 0 };
                    let visual_count = instances.len() + row_offset;

                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('h') | KeyCode::Esc | KeyCode::Left => {
                            app.screen = Screen::MainMenu;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.list_idx > 0 {
                                app.list_idx -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if app.list_idx < visual_count - 1 {
                                app.list_idx += 1;
                            }
                        }
                        KeyCode::Enter => {
                            if is_delete && app.list_idx == 0 {
                                // Delete all confirmation
                                let names: Vec<String> =
                                    instances.iter().map(|i| i.name.clone()).collect();
                                let count = names.len();
                                let msg = format!(
                                    "Delete ALL {} instance{}? This cannot be undone.",
                                    count,
                                    if count != 1 { "s" } else { "" }
                                );
                                app.screen = Screen::Confirm {
                                    message: msg,
                                    selected: 1, // default to No
                                    pending_action: PendingAction::DeleteAll(names),
                                };
                            } else {
                                let real_idx = app.list_idx - row_offset;
                                let inst_name = instances[real_idx].name.clone();
                                let act = action.clone();

                                match act.as_str() {
                                    "Enter" => {
                                        let inst_state = instances[real_idx].state.clone();
                                        leave_tui(&mut terminal);
                                        let _ = Command::new("clear").status();
                                        if inst_state != "RUNNING" {
                                            println!(
                                                "{}  Starting {}...",
                                                ICON_CREATE, inst_name
                                            );
                                            let _ = Command::new("incus")
                                                .args(["start", &inst_name])
                                                .status();
                                        }
                                        let _ = Command::new("incus")
                                            .args(["exec", &inst_name, "--", "bash"])
                                            .status();
                                        enter_tui(&mut terminal);
                                        app.screen = Screen::MainMenu;
                                    }
                                    "Stop" => {
                                        leave_tui(&mut terminal);
                                        let _ = Command::new("clear").status();
                                        let _ = Command::new("incus")
                                            .args(["stop", &inst_name])
                                            .status();
                                        print!("\nPress Enter to continue...");
                                        let _ = io::stdout().flush();
                                        let mut buf = String::new();
                                        let _ = io::stdin().read_line(&mut buf);
                                        enter_tui(&mut terminal);
                                        app.screen = Screen::MainMenu;
                                    }
                                    "Delete" => {
                                        leave_tui(&mut terminal);
                                        let _ = Command::new("clear").status();
                                        let _ = Command::new("incus")
                                            .args(["delete", "-f", &inst_name])
                                            .status();
                                        print!("\nPress Enter to continue...");
                                        let _ = io::stdout().flush();
                                        let mut buf = String::new();
                                        let _ = io::stdin().read_line(&mut buf);
                                        enter_tui(&mut terminal);
                                        app.screen = Screen::MainMenu;
                                    }
                                    _ => {
                                        app.screen = Screen::MainMenu;
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }

                Screen::Confirm {
                    selected,
                    pending_action,
                    ..
                } => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        app.screen = Screen::MainMenu;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if *selected > 0 {
                            *selected -= 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if *selected < 1 {
                            *selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        if *selected == 0 {
                            // Yes confirmed
                            let action = pending_action.clone();
                            match action {
                                PendingAction::DeleteAll(names) => {
                                    leave_tui(&mut terminal);
                                    let _ = Command::new("clear").status();
                                    for name in &names {
                                        let _ = Command::new("incus")
                                            .args(["delete", "-f", name])
                                            .status();
                                    }
                                    print!("\nPress Enter to continue...");
                                    let _ = io::stdout().flush();
                                    let mut buf = String::new();
                                    let _ = io::stdin().read_line(&mut buf);
                                    enter_tui(&mut terminal);
                                }
                                PendingAction::None => {}
                            }
                        }
                        app.screen = Screen::MainMenu;
                    }
                    _ => {}
                },

                Screen::Quit => break,
            }
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
