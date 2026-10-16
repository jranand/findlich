use crate::query::FileQuery;
use jiff::Timestamp;
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

pub struct IndexedFile {
    pub path: PathBuf,
    pub modified: Option<Timestamp>,
}

pub struct Index {
    files: Vec<IndexedFile>,
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry.depth() > 0
        && entry
            .file_name()
            .to_str()
            .is_some_and(|s| s.starts_with('.'))
}

impl Index {
    /// Walk the home folder once and remember every visible file.
    pub fn build() -> Index {
        let mut files = Vec::new();
        let Some(home) = dirs::home_dir() else {
            return Index { files };
        };

        for entry in WalkDir::new(&home)
            .into_iter()
            .filter_entry(|e| !is_hidden(e))
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let modified = entry
                .metadata()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| Timestamp::try_from(t).ok());

            files.push(IndexedFile {
                path: entry.into_path(),
                modified,
            });
        }

        // Sort once, newest first. Every search keeps this order for free.
        files.sort_by(|a, b| b.modified.cmp(&a.modified));
        Index { files }
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn search(&self, q: &FileQuery) -> Vec<&IndexedFile> {
        // Work out dates and folders once per search, not once per file.
        let range = q.time.range().map(|(s, e)| (s.timestamp(), e.timestamp()));
        let roots = q.scope.roots();

        self.files
            .iter()
            .filter(|f| roots.iter().any(|r| f.path.starts_with(r)))
            .filter(|f| q.kind.matches(&f.path))
            .filter(|f| q.matches_name(&f.path))
            .filter(|f| match range {
                None => true,
                Some((start, end)) => f.modified.is_some_and(|t| start <= t && t < end),
            })
            .collect()
    }
}
