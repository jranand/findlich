use crate::query::FileQuery;
use jiff::Timestamp;
use std::path::PathBuf;
use walkdir::{DirEntry, WalkDir};

fn is_hidden(entry: &DirEntry) -> bool {
    entry.depth() > 0
        && entry
            .file_name()
            .to_str()
            .map(|s| s.starts_with('.'))
            .unwrap_or(false)
}

pub fn search(q: &FileQuery) -> Vec<PathBuf> {
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
