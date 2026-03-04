import curses
import subprocess
import shutil
import argparse

WAZUH_MODE: bool = False


def _apply_resize(stdscr: curses.window) -> None:
    curses.update_lines_cols()


def _addstr(win: curses.window, *args) -> None:
    try:
        win.addstr(*args)
    except curses.error:
        pass


ICON_VM = "\uf0a0"
ICON_CREATE = "\uf055"
ICON_LIST = "\uf0ca"
ICON_ENTER = "\uf489"
ICON_STOP = "\uf28d"
ICON_DELETE = "\uf1f8"
ICON_EXIT = "\uf08b"
ICON_OS = "\uf17c"
ICON_INSTANCE = "\uf2d0"
ICON_ERROR = "\uf057"
ICON_RUNNING = "\uf444"
ICON_STOPPED = "\uf04d"
ICON_EMPTY = "\uf49e"

OS_GRID: dict[str, list[str]] = {
    "Ubuntu \uf31b": [
        "25.10 (questing)",
        "25.04 (plucky)",
        "24.04 (noble)",
        "22.04 (jammy)",
    ],
    "Debian \uf306": [
        "14 (forky)",
        "13 (trixie)",
        "12 (bookworm)",
        "11 (bullseye)",
    ],
    "CentOS \uf304": [
        "10-Stream",
        "9-Stream",
    ],
    "Fedora \uf30a": [
        "43",
        "42",
        "41",
    ],
    "AlmaLinux \uf31e": [
        "10 (RHEL-compat)",
        "9 (RHEL-compat)",
        "8 (RHEL-compat)",
    ],
    "Rocky \uf31e": [
        "10",
        "9",
        "8",
    ],
    "Amazon \uf270": [
        "2023",
    ],
    "openSUSE \uf314": [
        "16",
        "15",
    ],
}

IMAGE_ALIASES: dict[str, dict[str, str]] = {
    "Ubuntu \uf31b": {
        "25.10 (questing)": "images:ubuntu/questing",
        "25.04 (plucky)": "images:ubuntu/plucky",
        "24.04 (noble)": "images:ubuntu/noble",
        "22.04 (jammy)": "images:ubuntu/jammy",
    },
    "Debian \uf306": {
        "14 (forky)": "images:debian/14",
        "13 (trixie)": "images:debian/13",
        "12 (bookworm)": "images:debian/12",
        "11 (bullseye)": "images:debian/11",
    },
    "CentOS \uf304": {
        "10-Stream": "images:centos/10-Stream",
        "9-Stream": "images:centos/9-Stream",
    },
    "Fedora \uf30a": {
        "43": "images:fedora/43",
        "42": "images:fedora/42",
        "41": "images:fedora/41",
    },
    "AlmaLinux \uf31e": {
        "10 (RHEL-compat)": "images:almalinux/10",
        "9 (RHEL-compat)": "images:almalinux/9",
        "8 (RHEL-compat)": "images:almalinux/8",
    },
    "Rocky \uf31e": {
        "10": "images:rockylinux/10",
        "9": "images:rockylinux/9",
        "8": "images:rockylinux/8",
    },
    "Amazon \uf270": {
        "2023": "images:amazonlinux/2023",
    },
    "openSUSE \uf314": {
        "16": "images:opensuse/16.0",
        "15": "images:opensuse/15.6",
    },
}

MAIN_MENU: list[str] = ["Create", "List", "Enter", "Stop", "Delete", "Exit"]
MENU_ICONS: list[str] = [
    ICON_CREATE,
    ICON_LIST,
    ICON_ENTER,
    ICON_STOP,
    ICON_DELETE,
    ICON_EXIT,
]


def check_incus_available() -> bool:
    return shutil.which("incus") is not None


def check_incus_permissions() -> str | None:
    try:
        result = subprocess.run(
            ["incus", "list"], capture_output=True, text=True, timeout=5
        )
        if result.returncode != 0:
            stderr = result.stderr.strip()
            if "permission" in stderr.lower() or "unix.socket" in stderr.lower():
                user = subprocess.run(
                    ["whoami"], capture_output=True, text=True
                ).stdout.strip()
                return (
                    f"Permission denied on the incus socket.\n"
                    f"  Fix: sudo usermod -aG incus-admin {user} && newgrp incus-admin"
                )
            return f"incus error: {stderr}"
        return None
    except subprocess.TimeoutExpired:
        return "incus timed out — is the daemon running?"
    except Exception as exc:
        return str(exc)


def show_error_message(stdscr: curses.window, message: str) -> None:
    if not curses.isendwin():
        curses.endwin()
    print(f"\n{ICON_ERROR}  Error: {message}")
    input("Press Enter to continue...")


def display_confirm(stdscr: curses.window, message: str) -> bool:
    curses.curs_set(0)
    curses.init_pair(2, curses.COLOR_CYAN, -1)
    curses.init_pair(3, curses.COLOR_BLUE, -1)
    curses.init_pair(6, curses.COLOR_RED, -1)
    CURRENT: int = 1

    while True:
        _apply_resize(stdscr)
        stdscr.timeout(300)
        stdscr.clear()
        HEIGHT, WIDTH = stdscr.getmaxyx()

        _addstr(
            stdscr,
            2,
            2,
            f"{ICON_ERROR}  {message}",
            curses.color_pair(6) | curses.A_BOLD,
        )

        for IDX, LABEL in enumerate(["Yes", "No"]):
            Y = 5 + IDX
            if IDX == CURRENT:
                stdscr.attron(curses.color_pair(1) | curses.A_BOLD)
                _addstr(stdscr, Y, 4, f"  \u25b6 {LABEL}")
                stdscr.attroff(curses.color_pair(1) | curses.A_BOLD)
            else:
                _addstr(stdscr, Y, 4, f"    {LABEL}")

        HELP = "\uf062/\uf063 k/j: navigate  \u2502  Enter: confirm  \u2502  q: cancel"
        _addstr(stdscr, HEIGHT - 2, 2, HELP[: WIDTH - 4], curses.color_pair(3))
        stdscr.refresh()

        KEY = stdscr.getch()
        if KEY == curses.KEY_RESIZE or KEY == -1:
            continue
        elif KEY in [curses.KEY_UP, ord("k")] and CURRENT > 0:
            CURRENT -= 1
        elif KEY in [curses.KEY_DOWN, ord("j")] and CURRENT < 1:
            CURRENT += 1
        elif KEY in [curses.KEY_ENTER, 10, 13]:
            return CURRENT == 0
        elif KEY in [ord("q"), 27]:
            return False


def run_cli_commands(
    stdscr: curses.window, COMMANDS: list[list[str]], PAUSE: bool = True
) -> None:
    if not curses.isendwin():
        curses.endwin()
    subprocess.run("clear")
    for CMD in COMMANDS:
        subprocess.run(CMD)
    if PAUSE:
        input("\nPress Enter to continue...")
    stdscr.clear()
    stdscr.refresh()


def get_incus_instances() -> list[tuple[str, str, str, str, str]]:
    if not check_incus_available():
        return []
    try:
        result = subprocess.run(
            [
                "incus",
                "list",
                "-c",
                "n,config:image.os,config:image.release,s,4",
                "--format",
                "csv",
            ],
            capture_output=True,
            text=True,
            timeout=5,
        )
        rows: list[tuple[str, str, str, str, str]] = []
        for line in result.stdout.splitlines():
            parts = [p.strip() for p in line.split(",")]
            ipv4 = parts[4].split(" ")[0] if len(parts) > 4 else ""
            rows.append(
                (
                    parts[0] if len(parts) > 0 else "",
                    parts[1] if len(parts) > 1 else "",
                    parts[2] if len(parts) > 2 else "",
                    parts[3] if len(parts) > 3 else "",
                    ipv4,
                )
            )
        return rows
    except (subprocess.TimeoutExpired, Exception):
        return []


def display_main_menu(stdscr: curses.window) -> int:
    curses.curs_set(0)
    CURRENT_ROW: int = 0

    curses.init_pair(2, curses.COLOR_CYAN, -1)
    curses.init_pair(3, curses.COLOR_BLUE, -1)

    while True:
        _apply_resize(stdscr)
        stdscr.timeout(300)
        stdscr.clear()
        HEIGHT, WIDTH = stdscr.getmaxyx()

        TITLE_TEXT = f"{ICON_VM}  VM Manager"
        TITLE_COL = (WIDTH - len(TITLE_TEXT)) // 2
        _addstr(
            stdscr,
            1,
            max(0, TITLE_COL),
            TITLE_TEXT,
            curses.color_pair(2) | curses.A_BOLD,
        )

        for IDX, ROW in enumerate(MAIN_MENU):
            X: int = 4
            Y: int = 4 + (IDX * 2)
            LABEL = f"{MENU_ICONS[IDX]}  {ROW}"
            if IDX == CURRENT_ROW:
                stdscr.attron(curses.color_pair(1) | curses.A_BOLD)
                _addstr(stdscr, Y, X, f"  {LABEL}")
                stdscr.attroff(curses.color_pair(1) | curses.A_BOLD)
            else:
                _addstr(stdscr, Y, X, f"   {LABEL}")

        HELP_TEXT = "\uf062/\uf063 (k/j) navigate  \u2502  Enter select  \u2502  q quit"
        _addstr(stdscr, HEIGHT - 2, 2, HELP_TEXT, curses.color_pair(3))
        stdscr.refresh()

        KEY: int = stdscr.getch()
        if KEY == curses.KEY_RESIZE or KEY == -1:
            continue
        elif KEY in [curses.KEY_UP, ord("k")] and CURRENT_ROW > 0:
            CURRENT_ROW -= 1
        elif KEY in [curses.KEY_DOWN, ord("j")] and CURRENT_ROW < len(MAIN_MENU) - 1:
            CURRENT_ROW += 1
        elif KEY in [curses.KEY_ENTER, 10, 13]:
            return CURRENT_ROW
        elif KEY in [ord("q"), 27]:
            return -1


def display_list_menu(
    stdscr: curses.window,
    TITLE: str,
    OPTIONS: list[tuple[str, str, str, str, str]],
    DELETE_MODE: bool = False,
) -> int:
    curses.curs_set(0)
    CURRENT_ROW: int = 0
    curses.init_pair(2, curses.COLOR_CYAN, -1)
    curses.init_pair(3, curses.COLOR_BLUE, -1)
    curses.init_pair(5, curses.COLOR_GREEN, -1)
    curses.init_pair(6, curses.COLOR_RED, -1)

    VISUAL_COUNT: int = len(OPTIONS) + (1 if DELETE_MODE else 0)

    while True:
        _apply_resize(stdscr)
        stdscr.timeout(300)
        stdscr.clear()
        HEIGHT, WIDTH = stdscr.getmaxyx()

        _addstr(
            stdscr, 1, 2, f"{ICON_LIST}  {TITLE}", curses.color_pair(2) | curses.A_BOLD
        )

        if DELETE_MODE:
            Y_ALL: int = 4
            LABEL_ALL = f"{ICON_DELETE}  Delete All"
            if CURRENT_ROW == 0:
                stdscr.attron(curses.color_pair(6) | curses.A_BOLD)
                _addstr(stdscr, Y_ALL, 4, f"\u25b6 {LABEL_ALL}"[: WIDTH - 6])
                stdscr.attroff(curses.color_pair(6) | curses.A_BOLD)
            else:
                _addstr(
                    stdscr,
                    Y_ALL,
                    4,
                    f"  {LABEL_ALL}"[: WIDTH - 6],
                    curses.color_pair(6),
                )

        ROW_OFFSET: int = 1 if DELETE_MODE else 0
        for IDX, (NAME, _OS, _REL, STATE, _IP) in enumerate(OPTIONS):
            X: int = 4
            Y: int = 4 + ROW_OFFSET + IDX
            VISUAL_IDX = IDX + ROW_OFFSET
            STATUS_ICON = ICON_RUNNING if STATE == "RUNNING" else ICON_STOPPED
            STATUS_COLOR = (
                curses.color_pair(5) if STATE == "RUNNING" else curses.color_pair(6)
            )
            if VISUAL_IDX == CURRENT_ROW:
                HIGHLIGHT = curses.color_pair(1) | curses.A_BOLD
                _addstr(stdscr, Y, X, "\u25b6 ", HIGHLIGHT)
                _addstr(stdscr, STATUS_ICON, HIGHLIGHT)
                _addstr(stdscr, f"  {NAME}", HIGHLIGHT)
            else:
                _addstr(stdscr, Y, X, "  ")
                _addstr(stdscr, STATUS_ICON, STATUS_COLOR)
                _addstr(stdscr, f"  {NAME}")

        HELP_TEXT = (
            "\uf062/\uf063 k/j: navigate  \u2502  Enter: select  \u2502  q/h: back"
        )
        _addstr(stdscr, HEIGHT - 2, 2, HELP_TEXT[: WIDTH - 4], curses.color_pair(3))
        stdscr.refresh()

        KEY: int = stdscr.getch()
        if KEY == curses.KEY_RESIZE or KEY == -1:
            continue
        elif KEY in [curses.KEY_UP, ord("k")] and CURRENT_ROW > 0:
            CURRENT_ROW -= 1
        elif KEY in [curses.KEY_DOWN, ord("j")] and CURRENT_ROW < VISUAL_COUNT - 1:
            CURRENT_ROW += 1
        elif KEY in [curses.KEY_ENTER, 10, 13]:
            if DELETE_MODE and CURRENT_ROW == 0:
                return len(OPTIONS)
            return CURRENT_ROW - ROW_OFFSET
        elif KEY in [ord("q"), ord("h"), 27, curses.KEY_LEFT]:
            return -1


def display_instances_table(
    stdscr: curses.window, instances: list[tuple[str, str, str, str, str]]
) -> None:
    curses.curs_set(0)
    HEADERS = ("NAME", "OS", "RELEASE", "STATE", "IPv4")
    OFFSET = 0

    while True:
        _apply_resize(stdscr)
        stdscr.timeout(300)
        stdscr.clear()
        HEIGHT, WIDTH = stdscr.getmaxyx()

        col_w = [
            max(len(HEADERS[i]), max((len(row[i]) for row in instances), default=0))
            for i in range(5)
        ]

        USABLE = WIDTH - 4
        total = sum(col_w) + 16
        if total > USABLE:
            excess = total - USABLE
            col_w[0] = max(6, col_w[0] - excess // 2)
            col_w[4] = max(7, col_w[4] - (excess - excess // 2))

        def fmt_row(row: tuple[str, ...]) -> list[str]:
            return [str(row[i])[: col_w[i]].ljust(col_w[i]) for i in range(5)]

        def draw_sep(y: int) -> None:
            sep = "─" * (sum(col_w) + 16)
            _addstr(stdscr, y, 2, sep[: WIDTH - 4], curses.color_pair(3))

        _addstr(
            stdscr,
            1,
            2,
            f"{ICON_LIST}  Instances",
            curses.color_pair(2) | curses.A_BOLD,
        )

        if not instances:
            _addstr(
                stdscr, 3, 4, f"{ICON_EMPTY}  No instances found.", curses.color_pair(6)
            )
        else:
            draw_sep(3)
            hdr = fmt_row(HEADERS)  # type: ignore[arg-type]
            x = 2
            for i, h in enumerate(hdr):
                _addstr(stdscr, 4, x, h, curses.color_pair(4) | curses.A_BOLD)
                x += col_w[i] + 4
            draw_sep(5)

            LIST_TOP = 6
            LIST_HEIGHT = HEIGHT - LIST_TOP - 3
            visible = instances[OFFSET : OFFSET + LIST_HEIGHT]
            for idx, row in enumerate(visible):
                y = LIST_TOP + idx
                name, os_, rel, state, ip = row
                STATUS_ICON = ICON_RUNNING if state == "RUNNING" else ICON_STOPPED
                STATUS_COLOR = (
                    curses.color_pair(5) if state == "RUNNING" else curses.color_pair(6)
                )
                cells = fmt_row((name, os_, rel, state, ip))
                x = 2
                _addstr(stdscr, y, x, cells[0])
                x += col_w[0] + 4
                _addstr(stdscr, y, x, cells[1])
                x += col_w[1] + 4
                _addstr(stdscr, y, x, cells[2])
                x += col_w[2] + 4
                _addstr(stdscr, y, x, STATUS_ICON, STATUS_COLOR)
                _addstr(stdscr, f" {cells[3]}", STATUS_COLOR)
                x += col_w[3] + 4
                _addstr(stdscr, y, x, cells[4])
            draw_sep(LIST_TOP + len(visible))

            total_inst = len(instances)
            if total_inst > LIST_HEIGHT:
                scroll_hint = f" {OFFSET + 1}-{min(OFFSET + LIST_HEIGHT, total_inst)}/{total_inst} "
                _addstr(
                    stdscr,
                    LIST_TOP + len(visible),
                    WIDTH - len(scroll_hint) - 2,
                    scroll_hint,
                    curses.color_pair(3),
                )

        HELP = "\uf062/\uf063 k/j: scroll  \u2502  q: back"
        _addstr(stdscr, HEIGHT - 2, 2, HELP[: WIDTH - 4], curses.color_pair(3))
        stdscr.refresh()

        KEY = stdscr.getch()
        if KEY == curses.KEY_RESIZE or KEY == -1:
            continue
        if KEY in [ord("q"), ord("h"), 27, curses.KEY_LEFT]:
            return
        LIST_HEIGHT = HEIGHT - 9
        if KEY in [curses.KEY_DOWN, ord("j")] and OFFSET + LIST_HEIGHT < len(instances):
            OFFSET += 1
        elif KEY in [curses.KEY_UP, ord("k")] and OFFSET > 0:
            OFFSET -= 1


def display_grid_menu(stdscr: curses.window) -> tuple[str, str]:
    curses.curs_set(0)
    COLUMNS: list[str] = list(OS_GRID.keys())
    NUM_COLS: int = len(COLUMNS)
    CURRENT_COL: int = 0
    CURRENT_ROW: int = 0

    curses.init_pair(2, curses.COLOR_CYAN, -1)
    curses.init_pair(3, curses.COLOR_BLUE, -1)
    curses.init_pair(4, curses.COLOR_GREEN, -1)
    curses.init_pair(7, curses.COLOR_WHITE, -1)

    GLOBAL_BOX_W: int = 0
    GLOBAL_MAX_VERSIONS: int = 0

    for COL_NAME in COLUMNS:
        HDR_W: int = len(COL_NAME)
        VER_W: int = max(len(f"\u25b6 {V}") for V in OS_GRID[COL_NAME])
        COL_W: int = max(HDR_W, VER_W) + 4
        if COL_W > GLOBAL_BOX_W:
            GLOBAL_BOX_W = COL_W
        if len(OS_GRID[COL_NAME]) > GLOBAL_MAX_VERSIONS:
            GLOBAL_MAX_VERSIONS = len(OS_GRID[COL_NAME])

    GLOBAL_BOX_H: int = GLOBAL_MAX_VERSIONS + 2
    TOP_COUNT: int = (NUM_COLS + 1) // 2

    def draw_box(Y: int, X: int, W: int, H: int, TITLE: str, ACTIVE: bool) -> None:
        COLOR_BOX = curses.color_pair(4) if ACTIVE else curses.color_pair(7)
        COLOR_TITLE = curses.color_pair(4) | curses.A_BOLD

        _addstr(stdscr, Y, X, "╭" + "─" * (W - 2) + "╮", COLOR_BOX)
        for I in range(1, H - 1):
            _addstr(stdscr, Y + I, X, "│", COLOR_BOX)
            _addstr(stdscr, Y + I, X + W - 1, "│", COLOR_BOX)
        _addstr(stdscr, Y + H - 1, X, "╰" + "─" * (W - 2) + "╯", COLOR_BOX)
        _addstr(stdscr, Y, X + 2, f" {TITLE} ", COLOR_TITLE)

    while True:
        _apply_resize(stdscr)
        stdscr.timeout(300)
        stdscr.clear()
        HEIGHT, WIDTH = stdscr.getmaxyx()
        USABLE: int = WIDTH - 4

        if USABLE >= NUM_COLS * GLOBAL_BOX_W:
            LAYOUT = "single_row"
        elif USABLE >= TOP_COUNT * GLOBAL_BOX_W:
            LAYOUT = "two_row"
        else:
            LAYOUT = "narrow"

        MAIN_TITLE = f"{ICON_OS}  Select OS and Version"
        _addstr(stdscr, 1, 2, MAIN_TITLE, curses.color_pair(2) | curses.A_BOLD)

        if LAYOUT == "single_row":
            EXTRA: int = USABLE - (NUM_COLS * GLOBAL_BOX_W)
            GAP: int = EXTRA // NUM_COLS
            X_POS: int = 2 + (GAP // 2)

            for C_IDX, COL_NAME in enumerate(COLUMNS):
                IS_ACTIVE: bool = C_IDX == CURRENT_COL
                draw_box(3, X_POS, GLOBAL_BOX_W, GLOBAL_BOX_H, COL_NAME, IS_ACTIVE)

                for R_IDX, VERSION in enumerate(OS_GRID[COL_NAME]):
                    Y: int = 4 + R_IDX
                    if C_IDX == CURRENT_COL and R_IDX == CURRENT_ROW:
                        stdscr.attron(curses.color_pair(1) | curses.A_BOLD)
                        _addstr(
                            stdscr,
                            Y,
                            X_POS + 2,
                            f"\u25b6 {VERSION}"[: GLOBAL_BOX_W - 4],
                        )
                        stdscr.attroff(curses.color_pair(1) | curses.A_BOLD)
                    else:
                        _addstr(
                            stdscr, Y, X_POS + 2, f"  {VERSION}"[: GLOBAL_BOX_W - 4]
                        )
                X_POS += GLOBAL_BOX_W + GAP

        elif LAYOUT == "two_row":
            TOP_COLS_L = COLUMNS[:TOP_COUNT]
            BOT_COLS_L = COLUMNS[TOP_COUNT:]

            EXTRA_TWO: int = USABLE - (TOP_COUNT * GLOBAL_BOX_W)
            GAP_TWO: int = EXTRA_TWO // TOP_COUNT

            X_POS = 2 + (GAP_TWO // 2)
            for C_IDX, COL_NAME in enumerate(TOP_COLS_L):
                IS_ACTIVE: bool = C_IDX == CURRENT_COL
                draw_box(3, X_POS, GLOBAL_BOX_W, GLOBAL_BOX_H, COL_NAME, IS_ACTIVE)

                for R_IDX, VERSION in enumerate(OS_GRID[COL_NAME]):
                    Y = 4 + R_IDX
                    if C_IDX == CURRENT_COL and R_IDX == CURRENT_ROW:
                        stdscr.attron(curses.color_pair(1) | curses.A_BOLD)
                        _addstr(
                            stdscr,
                            Y,
                            X_POS + 2,
                            f"\u25b6 {VERSION}"[: GLOBAL_BOX_W - 4],
                        )
                        stdscr.attroff(curses.color_pair(1) | curses.A_BOLD)
                    else:
                        _addstr(
                            stdscr, Y, X_POS + 2, f"  {VERSION}"[: GLOBAL_BOX_W - 4]
                        )
                X_POS += GLOBAL_BOX_W + GAP_TWO

            BOT_START_Y: int = 3 + GLOBAL_BOX_H + 1
            X_POS = 2 + (GAP_TWO // 2)

            for C_IDX_BOT, COL_NAME in enumerate(BOT_COLS_L):
                GLOBAL_C_IDX: int = TOP_COUNT + C_IDX_BOT
                IS_ACTIVE: bool = GLOBAL_C_IDX == CURRENT_COL

                draw_box(
                    BOT_START_Y, X_POS, GLOBAL_BOX_W, GLOBAL_BOX_H, COL_NAME, IS_ACTIVE
                )

                for R_IDX, VERSION in enumerate(OS_GRID[COL_NAME]):
                    Y = BOT_START_Y + 1 + R_IDX
                    if GLOBAL_C_IDX == CURRENT_COL and R_IDX == CURRENT_ROW:
                        stdscr.attron(curses.color_pair(1) | curses.A_BOLD)
                        _addstr(
                            stdscr,
                            Y,
                            X_POS + 2,
                            f"\u25b6 {VERSION}"[: GLOBAL_BOX_W - 4],
                        )
                        stdscr.attroff(curses.color_pair(1) | curses.A_BOLD)
                    else:
                        _addstr(
                            stdscr, Y, X_POS + 2, f"  {VERSION}"[: GLOBAL_BOX_W - 4]
                        )
                X_POS += GLOBAL_BOX_W + GAP_TWO

        else:
            COL_NAME = COLUMNS[CURRENT_COL]
            draw_box(
                3,
                2,
                WIDTH - 4,
                GLOBAL_BOX_H,
                f"{COL_NAME}  ({CURRENT_COL + 1}/{NUM_COLS})",
                True,
            )

            for R_IDX, VERSION in enumerate(OS_GRID[COL_NAME]):
                Y: int = 4 + R_IDX
                if R_IDX == CURRENT_ROW:
                    stdscr.attron(curses.color_pair(1) | curses.A_BOLD)
                    _addstr(stdscr, Y, 4, f"\u25b6 {VERSION}"[: WIDTH - 8])
                    stdscr.attroff(curses.color_pair(1) | curses.A_BOLD)
                else:
                    _addstr(stdscr, Y, 4, f"  {VERSION}"[: WIDTH - 8])

        HELP = (
            "\uf060/\uf061 h/l: distro  "
            "\u2502  \uf062/\uf063 k/j: version  "
            "\u2502  Enter: select  \u2502  q: back"
        )
        _addstr(stdscr, HEIGHT - 2, 2, HELP[: WIDTH - 4], curses.color_pair(3))
        stdscr.refresh()

        KEY: int = stdscr.getch()

        if KEY == curses.KEY_RESIZE or KEY == -1:
            continue
        elif KEY in [curses.KEY_UP, ord("k")]:
            if CURRENT_ROW > 0:
                CURRENT_ROW -= 1
            elif LAYOUT == "two_row" and CURRENT_COL >= TOP_COUNT:
                CURRENT_COL -= TOP_COUNT
                CURRENT_ROW = len(OS_GRID[COLUMNS[CURRENT_COL]]) - 1
        elif KEY in [curses.KEY_DOWN, ord("j")]:
            if CURRENT_ROW < len(OS_GRID[COLUMNS[CURRENT_COL]]) - 1:
                CURRENT_ROW += 1
            elif LAYOUT == "two_row" and CURRENT_COL < TOP_COUNT:
                NEXT_COL = CURRENT_COL + TOP_COUNT
                if NEXT_COL < NUM_COLS:
                    CURRENT_COL = NEXT_COL
                    CURRENT_ROW = 0
        elif KEY in [curses.KEY_LEFT, ord("h")] and CURRENT_COL > 0:
            CURRENT_COL -= 1
            if CURRENT_ROW >= len(OS_GRID[COLUMNS[CURRENT_COL]]):
                CURRENT_ROW = len(OS_GRID[COLUMNS[CURRENT_COL]]) - 1
        elif KEY in [curses.KEY_RIGHT, ord("l")] and CURRENT_COL < NUM_COLS - 1:
            CURRENT_COL += 1
            if CURRENT_ROW >= len(OS_GRID[COLUMNS[CURRENT_COL]]):
                CURRENT_ROW = len(OS_GRID[COLUMNS[CURRENT_COL]]) - 1
        elif KEY in [curses.KEY_ENTER, 10, 13]:
            return COLUMNS[CURRENT_COL], OS_GRID[COLUMNS[CURRENT_COL]][CURRENT_ROW]
        elif KEY in [ord("q"), 27]:
            return "", ""


def handle_create(stdscr: curses.window) -> None:
    if not check_incus_available():
        show_error_message(
            stdscr, "incus command not found. Please install LXD/Incus first."
        )
        return

    SELECTED_DISTRO: str
    SELECTED_VERSION: str
    SELECTED_DISTRO, SELECTED_VERSION = display_grid_menu(stdscr)
    if not SELECTED_DISTRO or not SELECTED_VERSION:
        return

    curses.echo()
    curses.curs_set(1)
    stdscr.clear()
    HEIGHT, WIDTH = stdscr.getmaxyx()
    curses.init_pair(2, curses.COLOR_CYAN, -1)

    TITLE = f"{ICON_CREATE}  Creating {SELECTED_DISTRO} {SELECTED_VERSION}"
    _addstr(stdscr, 2, 2, TITLE, curses.color_pair(2) | curses.A_BOLD)

    PROMPT = f"{ICON_INSTANCE}  Instance name: "
    _addstr(stdscr, 4, 2, PROMPT, curses.A_NORMAL)
    stdscr.refresh()

    MAX_INPUT_WIDTH = min(50, WIDTH - len(PROMPT) - 4)
    INSTANCE_NAME_BYTES: bytes = stdscr.getstr(4, 2 + len(PROMPT), MAX_INPUT_WIDTH)
    INSTANCE_NAME: str = INSTANCE_NAME_BYTES.decode("utf-8").strip()

    curses.noecho()
    curses.curs_set(0)

    if not INSTANCE_NAME:
        return

    IMAGE_PATH: str = IMAGE_ALIASES[SELECTED_DISTRO][SELECTED_VERSION]
    COMMANDS_TO_RUN: list[list[str]] = [["incus", "launch", IMAGE_PATH, INSTANCE_NAME]]

    WAIT_NETWORK_CMD: list[str] = [
        "incus",
        "exec",
        INSTANCE_NAME,
        "--",
        "sh",
        "-c",
        "for i in $(seq 1 30); do ping -c 1 -W 1 1.1.1.1 >/dev/null 2>&1 && break || sleep 1; done",
    ]
    COMMANDS_TO_RUN.append(WAIT_NETWORK_CMD)

    if "Ubuntu" in SELECTED_DISTRO or "Debian" in SELECTED_DISTRO:
        COMMANDS_TO_RUN.append(
            [
                "incus",
                "exec",
                INSTANCE_NAME,
                "--",
                "env",
                "DEBIAN_FRONTEND=noninteractive",
                "apt-get",
                "update",
                "-y",
            ]
        )
    elif any(
        d in SELECTED_DISTRO
        for d in ("CentOS", "Fedora", "AlmaLinux", "Rocky", "Amazon")
    ):
        COMMANDS_TO_RUN.append(
            ["incus", "exec", INSTANCE_NAME, "--", "dnf", "makecache"]
        )
    elif "openSUSE" in SELECTED_DISTRO:
        COMMANDS_TO_RUN.append(
            ["incus", "exec", INSTANCE_NAME, "--", "zypper", "refresh"]
        )

    # Wazuh build deps + MOTD (only with --wazuh flag)
    if WAZUH_MODE:
        if "Ubuntu" in SELECTED_DISTRO or "Debian" in SELECTED_DISTRO:
            COMMANDS_TO_RUN.append(
                [
                    "incus",
                    "exec",
                    INSTANCE_NAME,
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
                ]
            )
        elif any(
            d in SELECTED_DISTRO
            for d in ("CentOS", "Fedora", "AlmaLinux", "Rocky", "Amazon")
        ):
            COMMANDS_TO_RUN.append(
                [
                    "incus",
                    "exec",
                    INSTANCE_NAME,
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
                ]
            )
        elif "openSUSE" in SELECTED_DISTRO:
            COMMANDS_TO_RUN.append(
                [
                    "incus",
                    "exec",
                    INSTANCE_NAME,
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
                ]
            )

        MOTD = (
            "╔══════════════════════════════════════════════════════════════╗\n"
            "║                  Wazuh Development Container                ║\n"
            "╠══════════════════════════════════════════════════════════════╣\n"
            "║  Build dependencies are pre-installed.                      ║\n"
            "║                                                             ║\n"
            "║  Quick install (all-in-one):                                ║\n"
            "║    curl -sO https://packages.wazuh.com/4.14/wazuh-install.sh║\n"
            "║    sudo bash ./wazuh-install.sh -a                          ║\n"
            "║                                                             ║\n"
            "║  Docs: https://documentation.wazuh.com                      ║\n"
            "╚══════════════════════════════════════════════════════════════╝\n"
        )
        MOTD_CMD = f"printf '{MOTD}' > /etc/motd"
        COMMANDS_TO_RUN.append(
            ["incus", "exec", INSTANCE_NAME, "--", "sh", "-c", MOTD_CMD]
        )

    COMMANDS_TO_RUN.append(["incus", "exec", INSTANCE_NAME, "--", "bash"])
    run_cli_commands(stdscr, COMMANDS_TO_RUN, PAUSE=False)


def handle_instance_action(stdscr: curses.window, ACTION: str) -> None:
    if not check_incus_available():
        show_error_message(
            stdscr, "incus command not found. Please install LXD/Incus first."
        )
        return

    INSTANCES: list[tuple[str, str, str, str, str]] = get_incus_instances()
    if not INSTANCES:
        run_cli_commands(stdscr, [["echo", f"{ICON_EMPTY}  No instances found."]])
        return

    IS_DELETE = ACTION == "Delete"
    SELECTED_IDX: int = display_list_menu(
        stdscr, f"Select instance to {ACTION}:", INSTANCES, DELETE_MODE=IS_DELETE
    )
    if SELECTED_IDX == -1:
        return

    if IS_DELETE and SELECTED_IDX == len(INSTANCES):
        NAMES = [inst[0] for inst in INSTANCES]
        COUNT = len(NAMES)
        if not display_confirm(
            stdscr,
            f"Delete ALL {COUNT} instance{'s' if COUNT != 1 else ''}? This cannot be undone.",
        ):
            return
        COMMANDS = [["incus", "delete", "-f", name] for name in NAMES]
        run_cli_commands(stdscr, COMMANDS)
        return

    SELECTED_INSTANCE: str = INSTANCES[SELECTED_IDX][0]
    SELECTED_STATE: str = INSTANCES[SELECTED_IDX][3]
    if ACTION == "Enter":
        COMMANDS: list[list[str]] = []
        if SELECTED_STATE != "RUNNING":
            COMMANDS.append(["echo", f"{ICON_CREATE}  Starting {SELECTED_INSTANCE}..."])
            COMMANDS.append(["incus", "start", SELECTED_INSTANCE])
        COMMANDS.append(["incus", "exec", SELECTED_INSTANCE, "--", "bash"])
        run_cli_commands(stdscr, COMMANDS, PAUSE=False)
    elif ACTION == "Stop":
        run_cli_commands(stdscr, [["incus", "stop", SELECTED_INSTANCE]])
    elif ACTION == "Delete":
        run_cli_commands(stdscr, [["incus", "delete", "-f", SELECTED_INSTANCE]])


def main_app(stdscr: curses.window) -> None:
    curses.start_color()
    curses.use_default_colors()

    curses.init_pair(1, curses.COLOR_BLACK, curses.COLOR_WHITE)
    curses.init_pair(2, curses.COLOR_CYAN, -1)
    curses.init_pair(3, curses.COLOR_BLUE, -1)
    curses.init_pair(4, curses.COLOR_GREEN, -1)
    curses.init_pair(5, curses.COLOR_GREEN, -1)
    curses.init_pair(6, curses.COLOR_RED, -1)

    if not check_incus_available():
        show_error_message(
            stdscr, "incus command not found. Please install LXD/Incus first."
        )
        return
    perm_error = check_incus_permissions()
    if perm_error:
        show_error_message(stdscr, perm_error)
        return

    while True:
        SELECTED_MAIN: int = display_main_menu(stdscr)
        if SELECTED_MAIN == -1 or MAIN_MENU[SELECTED_MAIN] == "Exit":
            break
        ACTION: str = MAIN_MENU[SELECTED_MAIN]
        if ACTION == "Create":
            handle_create(stdscr)
        elif ACTION == "List":
            display_instances_table(stdscr, get_incus_instances())
        else:
            handle_instance_action(stdscr, ACTION)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Incus container manager TUI")
    parser.add_argument(
        "--wazuh",
        action="store_true",
        help="Enable Wazuh mode: pre-install build dependencies and write /etc/motd with Wazuh quickstart",
    )
    args = parser.parse_args()
    WAZUH_MODE = args.wazuh
    curses.wrapper(main_app)
