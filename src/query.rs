use jiff::{ToSpan, Zoned};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum FileKind {
    Pdf,
    Pptx,
    Docx,
    Markdown,
    Images,
    Any,
}

#[derive(Debug)]
pub enum Scope {
    UserFolders,
    Everywhere,
}

#[derive(Debug)]
pub enum TimeRange {
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
pub struct FileQuery {
    pub kind: FileKind,
    pub time: TimeRange,
    pub scope: Scope,
}

impl Scope {
    pub fn roots(&self) -> Vec<PathBuf> {
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

    pub fn matches(&self, path: &Path) -> bool {
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
    pub fn range(&self) -> Option<(Zoned, Zoned)> {
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
                now.checked_sub(i64::from(*n).days()).expect("date math"),
                tomorrow,
            ),
        };
        Some(range)
    }
}

impl FileQuery {
    // The "Show specs" idea: every query can explain itself as a real command.
    pub fn to_find_command(&self) -> String {
        let roots: Vec<String> = self
            .scope
            .roots()
            .iter()
            .map(|p| format!("'{}'", p.display()))
            .collect();

        let mut cmd = format!("find {} -type f -not -path '*/.*'", roots.join(" "));

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
