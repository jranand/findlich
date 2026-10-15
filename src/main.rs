use jiff::{Timestamp, ToSpan, Zoned};
use std::path::{Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Debug)]
enum FileKind {
    Pdf,
    Pptx,
    Docx,
    Markdown,
    Images,
    Any,
}

#[derive(Debug)]
enum Scope {
    UserFolders,
    Everywhere,
}

#[derive(Debug)]
enum TimeRange {
    Any,
    Today,
    Yesterday,
    ThisWeek,
    LastWeek,
    ThisMonth,
    LastMonth,
    PastDays(u32),
}

#[derive(Debug)]
struct FileQuery {
    kind: FileKind,
    time: TimeRange,
    scope: Scope,
}

impl Scope {
    fn roots(&self) -> Vec<PathBuf> {
        match self {
            Scope::Everywhere => dirs::home_dir().into_iter().collect(),
            Scope::UserFolders => [
                dirs::desktop_dir(),
                dirs::document_dir(),
                dirs::download_dir(),
                dirs::picture_dir(),
                dirs::video_dir(),
                dirs::audio_dir(),
            ]
            .into_iter()
            .flatten()
            .collect(),
        }
    }
}

impl FileKind {
    fn extensions(&self) -> &'static [&'static str] {
        match self {
            FileKind::Pdf => &["pdf"],
            FileKind::Pptx => &["pptx"],
            FileKind::Docx => &["docx"],
            FileKind::Markdown => &["md"],
            FileKind::Images => &["jpg", "jpeg", "png", "webp"],
            FileKind::Any => &[],
        }
    }

    fn matches(&self, path: &Path) -> bool {
        let exts = self.extensions();
        if exts.is_empty() {
            return true; // Any: every file matches
        }
        match path.extension().and_then(|e| e.to_str()) {
            Some(ext) => exts.iter().any(|x| x.eq_ignore_ascii_case(ext)),
            None => false, // file has no extension
        }
    }
}

impl TimeRange {
    /// Turn a period into exact (start, end) moments in local time.
    /// None means "no time limit".
    fn range(&self) -> Option<(Zoned, Zoned)> {
        let now = Zoned::now();
        let today = now.start_of_day().expect("date math");
        let tomorrow = today.checked_add(1.day()).expect("date math");
        let monday = today
            .checked_sub(today.weekday().to_monday_zero_offset().days())
            .expect("date math");
        let month_start = today.first_of_month().expect("date math");

        let range = match self {
            TimeRange::Any => return None,
            TimeRange::Today => (today, tomorrow),
            TimeRange::Yesterday => (today.checked_sub(1.day()).expect("date math"), today),
            TimeRange::ThisWeek => (monday, tomorrow),
            TimeRange::LastWeek => (monday.checked_sub(1.week()).expect("date math"), monday),
            TimeRange::ThisMonth => (month_start, tomorrow),
            TimeRange::LastMonth => (
                month_start.checked_sub(1.month()).expect("date math"),
                month_start,
            ),
            TimeRange::PastDays(n) => (
                // ← new
                now.checked_sub(i64::from(*n).days()).expect("date math"),
                tomorrow,
            ),
        };
        Some(range)
    }
}

impl FileQuery {
    // The "Show specs" idea: every query can explain itself as a real command.
    fn to_find_command(&self) -> String {
        let mut cmd = String::from("find ~ -type f -not -path '*/.*'");

        let exts = self.kind.extensions();
        if !exts.is_empty() {
            let parts: Vec<String> = exts.iter().map(|e| format!("-iname '*.{e}'")).collect();
            cmd.push_str(&format!(" \\( {} \\)", parts.join(" -o ")));
        }
        if let Some((start, end)) = self.time.range() {
            cmd.push_str(&format!(
                " -newermt '{}' ! -newermt '{}'",
                start.strftime("%Y-%m-%d %H:%M"),
                end.strftime("%Y-%m-%d %H:%M"),
            ));
        }

        cmd
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.depth() > 0
        && entry
            .file_name()
            .to_str()
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
}

fn search(q: &FileQuery) -> Vec<PathBuf> {
    let mut results = Vec::new();

    // Work out the dates once, before walking thousands of files.
    let range = q.time.range().map(|(s, e)| (s.timestamp(), e.timestamp()));

    for root in q.scope.roots() {
        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() || !q.kind.matches(entry.path()) {
                continue;
            }

            if let Some((start, end)) = range {
                let Ok(meta) = entry.metadata() else { continue };
                let Ok(modified) = meta.modified() else {
                    continue;
                };
                let Ok(t) = Timestamp::try_from(modified) else {
                    continue;
                };
                if t < start || t >= end {
                    continue;
                }
            }

            results.push(entry.into_path());
        }
    }
    results
}

fn parse(input: &str) -> FileQuery {
    let text = input.to_lowercase();
    let words: Vec<&str> = text.split_whitespace().collect();

    let mut kind = FileKind::Any;
    let mut time = TimeRange::Any;
    let mut scope = Scope::UserFolders;
    let mut number: Option<u32> = None;
    let mut prev = "";

    for &word in &words {
        // File type: one word is enough.
        match word {
            "pdf" | "pdfs" => kind = FileKind::Pdf,
            "doc" | "docs" | "docx" => kind = FileKind::Docx,
            "md" | "markdown" => kind = FileKind::Markdown,
            "jpg" | "jpeg" | "png" | "webp" | "image" | "images" | "photos" | "pictures" => {
                kind = FileKind::Images
            }
            "pptx" | "slides" | "presentation" | "presentations" => kind = FileKind::Pptx,
            "everywhere" | "anywhere" => scope = Scope::Everywhere,
            _ => {}
        }

        // Time: look at (previous word, this word).
        match (prev, word) {
            (_, "today") => time = TimeRange::Today,
            (_, "yesterday") => time = TimeRange::Yesterday,
            ("last", "week") => time = TimeRange::LastWeek,
            ("last", "month") => time = TimeRange::LastMonth,
            (_, "week") => time = TimeRange::ThisWeek,
            (_, "month") => time = TimeRange::ThisMonth,
            (_, "recent") => time = TimeRange::PastDays(7),
            _ => {}
        }

        if let Some(n) = number {
            match word {
                "day" | "days" => time = TimeRange::PastDays(n),
                "week" | "weeks" => time = TimeRange::PastDays(n * 7),
                _ => {}
            }
        }

        number = word.parse::<u32>().ok();

        prev = word;
    }

    FileQuery { kind, time, scope }
}

fn main() {
    let input = std::env::args().skip(1).collect::<Vec<_>>().join(" ");

    let q = parse(&input);
    println!("{q:?}");
    println!("  → {}\n", q.to_find_command());

    let results = search(&q);
    for path in results.iter().take(20) {
        println!("{}", path.display());
    }
    println!("\n{} files found", results.len());
}
