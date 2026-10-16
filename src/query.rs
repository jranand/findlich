use jiff::{ToSpan, Zoned};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileKind {
    Pdf,
    Pptx,
    Docx,
    Markdown,
    Images,
    Ext(&'static str), 
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UserFolder {
    Desktop,
    Documents,
    Downloads,
    Pictures,
    Music,
    Videos,
}

#[derive(Debug)]
pub enum Scope {
    UserFolders,              // all standard folders (default)
    Folders(Vec<UserFolder>), // only the ones the user named
    Everywhere,               // all of ~
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
    pub kinds: Vec<FileKind>,
    pub time: TimeRange,
    pub scope: Scope,
    pub name_terms: Vec<String>,
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

impl UserFolder {
    pub const ALL: [UserFolder; 6] = [
        UserFolder::Desktop,
        UserFolder::Documents,
        UserFolder::Downloads,
        UserFolder::Pictures,
        UserFolder::Music,
        UserFolder::Videos,
    ];

    /// Where this folder really is, from the user's XDG settings.
    pub fn path(self) -> Option<PathBuf> {
        match self {
            UserFolder::Desktop => dirs::desktop_dir(),
            UserFolder::Documents => dirs::document_dir(),
            UserFolder::Downloads => dirs::download_dir(),
            UserFolder::Pictures => dirs::picture_dir(),
            UserFolder::Music => dirs::audio_dir(),
            UserFolder::Videos => dirs::video_dir(),
        }
    }
}

impl Scope {
    pub fn roots(&self) -> Vec<PathBuf> {
        let mut roots: Vec<PathBuf> = match self {
            Scope::Everywhere => dirs::home_dir().into_iter().collect(),
            Scope::Folders(folders) => folders.iter().filter_map(|f| f.path()).collect(),
            Scope::UserFolders => {
                let home = dirs::home_dir();
                UserFolder::ALL
                    .iter()
                    .filter_map(|f| f.path())
                    .filter(|p| home.as_ref() != Some(p)) // never the whole home
                    .collect()
            }
        };
        roots.sort();
        roots.dedup();
        roots
    }
}

impl FileKind {
    fn extensions(&self) -> &[&'static str] {
        match self {
            FileKind::Pdf => &["pdf"],
            FileKind::Pptx => &["pptx", "ppt", "odp"],
            FileKind::Docx => &["docx", "doc", "odt"],
            FileKind::Markdown => &["md"],
            FileKind::Images => &["jpg", "jpeg", "png", "webp", "gif", "heic", "svg"],
            FileKind::Ext(e) => std::slice::from_ref(e),
        }
    }

    pub fn matches(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|ext| {
                self.extensions()
                    .iter()
                    .any(|x| x.eq_ignore_ascii_case(ext))
            })
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
            .map(|p| shell_quote(&p.display().to_string()))
            .collect();

        let mut cmd = format!("find {} -type f -not -path '*/.*'", roots.join(" "));

        let exts: Vec<&str> = self
            .kinds
            .iter()
            .flat_map(|k| k.extensions().iter().copied())
            .collect();
        if !exts.is_empty() {
            let parts: Vec<String> = exts.iter().map(|e| format!("-iname '*.{e}'")).collect();
            cmd.push_str(&format!(" \\( {} \\)", parts.join(" -o ")));
        }

        for term in &self.name_terms {
            cmd.push_str(&format!(" -iname '*{term}*'"));
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

    pub fn matches_kind(&self, path: &Path) -> bool {
        self.kinds.is_empty() || self.kinds.iter().any(|k| k.matches(path))
    }

    pub fn matches_name(&self, path: &Path) -> bool {
        if self.name_terms.is_empty() {
            return true;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            return false;
        };
        let name = name.to_lowercase();
        self.name_terms.iter().all(|t| name.contains(t.as_str()))
    }
}
