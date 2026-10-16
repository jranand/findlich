# Findlich
**Spotlight-style search for Linux, in plain language. Just ask -  "pdf from last week".**

The power of the Linux terminal, in plain language.

Type `pdf files last week` or `photos yesterday` and Findlich finds them. Every search also shows the real shell command behind it, so you always know exactly what it did and can learn the terminal along the way.

> **Early development.** The basics work; expect rough edges and breaking changes.

## What works now

- **Natural-language file search**: file types, time periods and name words in any order
- **Native GTK4 / libadwaita window**: search on Enter, file-type icons, Esc to close
- **Command-line version**: the same search from your terminal
- **Show specs**: every search displays the equivalent `find` command

## Query examples

| You type | Finds |
|---|---|
| `pdf last week` | PDFs modified last Monday–Sunday |
| `photos yesterday` | images from yesterday |
| `docs this month` | .docx / .doc / .odt files since the 1st |
| `slides past 3 months` | presentations from the last 90 days |
| `invoice 2024 pdf` | PDFs with "invoice" and "2024" in the name |
| `notes everywhere` | files named "notes" anywhere in your home folder |

**File types:** pdf, docs, slides, markdown, images
**Calendar periods:** today, yesterday, this/last week, this/last month
**Rolling windows:** `3 days`, `past 2 weeks`, `last 6 months`, `1 year`, `recent`
**Where:** your Desktop, Documents, Downloads, Pictures, Music and Videos by default; add `everywhere` to search all of `~`

Words Findlich doesn't recognize as a type or time are matched against file names. Hidden folders are skipped.

## Principles

- **Local only.** No network, no accounts, no telemetry.
- **Transparent.** Every action shows the real command behind it.
- **Lean.** Light on CPU and memory, because those belong to you.
- **Native.** Follows your desktop's theme, icons and dark mode.
- **Linux first.** Built for Linux, not ported to it.

## Building

### Dependencies

Fedora:

```bash
sudo dnf install gtk4-devel libadwaita-devel gcc
```

Debian / Ubuntu:

```bash
sudo apt install libgtk-4-dev libadwaita-1-dev build-essential
```

Arch:

```bash
sudo pacman -S gtk4 libadwaita base-devel
```

You also need a recent Rust toolchain ([rustup.rs](https://rustup.rs)).

### Run

```bash
# Window
cargo run --bin gui

# Terminal
cargo run --bin findlich -- pdf last week
```

## Roadmap

- [ ] Search as you type, with a background index
- [ ] Open files, plus a details view (type, size, dates, permissions)
- [ ] Apps in results
- [ ] Quick settings: Wi-Fi, Bluetooth, dark mode, volume, brightness
- [ ] Global hotkey (works on any desktop)
- [ ] Optional local AI for fuzzier requests, fully offline

## Contributing

Issues and ideas are welcome. Findlich is young, so please open an issue before starting a larger change.
