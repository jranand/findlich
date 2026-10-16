use crate::query::{FileQuery, Resolved, UserFolder};
use ignore::WalkBuilder;
use jiff::Timestamp;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Dependency folders: machine-managed wherever they appear.
const DEPENDENCY_DIRS: &[&str] = &[
    "node_modules",
    "__pycache__",
    "bower_components",
    "site-packages",
];

/// Tool data that lives directly in the home folder (paths relative to ~).
const TOOL_DIRS: &[&str] = &["go/pkg", "snap", "miniconda3", "anaconda3", "Android/Sdk"];

/// Generic names, skipped only when a project file sits next to them.
const BUILD_DIRS: &[(&str, &[&str])] = &[
    ("target", &["Cargo.toml"]),
    (
        "build",
        &[
            "CMakeLists.txt",
            "meson.build",
            "package.json",
            "build.gradle",
            "pyproject.toml",
            "setup.py",
        ],
    ),
    ("dist", &["package.json", "pyproject.toml", "setup.py"]),
    ("vendor", &["go.mod", "composer.json"]),
];

/// File types nobody opens by hand: compiled output, partial downloads.
const MACHINE_EXTS: &[&str] = &[
    "o",
    "a",
    "so",
    "obj",
    "pyc",
    "pyo",
    "class",
    "map",
    "tmp",
    "swp",
    "part",
    "crdownload",
];

pub struct IndexedFile {
    pub path: PathBuf,
    pub modified: Option<Timestamp>,
}

pub struct Index {
    files: Vec<IndexedFile>,
    skipped: BTreeMap<&'static str, usize>,
}

/// The first `limit` matches (newest first), plus how many matched in total.
pub struct SearchResult<'a> {
    pub matches: Vec<&'a IndexedFile>,
    pub total: usize,
}

fn skip_dir_reason(path: &Path, home: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;

    if DEPENDENCY_DIRS.contains(&name) {
        return Some("dependencies");
    }
    if let Ok(rel) = path.strip_prefix(home)
        && TOOL_DIRS.iter().any(|d| rel == Path::new(d))
    {
        return Some("tool data");
    }
    if path.join("CACHEDIR.TAG").exists() {
        return Some("cache");
    }
    if path.join("pyvenv.cfg").exists() {
        return Some("python environment");
    }
    for (dir, markers) in BUILD_DIRS {
        if name == *dir
            && let Some(parent) = path.parent()
            && markers.iter().any(|m| parent.join(m).exists())
        {
            return Some("build output");
        }
    }
    None
}

fn is_machine_file(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| MACHINE_EXTS.iter().any(|m| m.eq_ignore_ascii_case(e)))
}

impl Index {
    /// Walk home (and any user folders kept elsewhere) once.
    pub fn build() -> Index {
        let mut files = Vec::new();
        let Some(home) = dirs::home_dir() else {
            return Index {
                files,
                skipped: BTreeMap::new(),
            };
        };

        let mut walker = WalkBuilder::new(&home);

        // User folders that live outside home, e.g. Documents on another drive.
        for folder in UserFolder::ALL {
            if let Some(path) = folder.path()
                && !path.starts_with(&home)
                && path.is_dir()
            {
                walker.add(path);
            }
        }

        walker
            .hidden(true)
            .git_ignore(true)
            .require_git(false)
            .git_global(false)
            .git_exclude(false)
            .parents(false)
            .add_custom_ignore_filename(".hidden");

        if let Some(config) = dirs::config_dir() {
            let user_rules = config.join("findlich/ignore");
            if user_rules.exists()
                && let Some(err) = walker.add_ignore(&user_rules)
            {
                eprintln!("findlich: problem reading {}: {err}", user_rules.display());
            }
        }

        let skipped = Arc::new(Mutex::new(BTreeMap::new()));
        {
            let skipped = Arc::clone(&skipped);
            let home = home.clone();
            walker.filter_entry(move |entry| {
                let is_dir = entry.file_type().is_some_and(|t| t.is_dir());
                if !is_dir || entry.depth() == 0 {
                    return true;
                }
                match skip_dir_reason(entry.path(), &home) {
                    Some(reason) => {
                        *skipped.lock().unwrap().entry(reason).or_insert(0) += 1;
                        false
                    }
                    None => true,
                }
            });
        }

        let mut unreadable = 0;
        for result in walker.build() {
            // A folder we're not allowed to read: count it, don't hide it.
            let entry = match result {
                Ok(entry) => entry,
                Err(_) => {
                    unreadable += 1;
                    continue;
                }
            };
            if !entry.file_type().is_some_and(|t| t.is_file()) || is_machine_file(entry.path()) {
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

        files.sort_by_key(|f| std::cmp::Reverse(f.modified));

        let mut skipped = std::mem::take(&mut *skipped.lock().unwrap());
        if unreadable > 0 {
            skipped.insert("unreadable", unreadable);
        }
        Index { files, skipped }
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn skipped(&self) -> &BTreeMap<&'static str, usize> {
        &self.skipped
    }

    /// One pass over the index: keep the first `limit` matches, count them all.
    pub fn search(&self, q: &FileQuery, plan: &Resolved, limit: usize) -> SearchResult<'_> {
        let range = plan
            .range
            .as_ref()
            .map(|(s, e)| (s.timestamp(), e.timestamp()));

        let mut matches = Vec::with_capacity(limit);
        let mut total = 0;

        for f in &self.files {
            let wanted = plan.roots.iter().any(|r| f.path.starts_with(r))
                && q.matches_kind(&f.path)
                && q.matches_name(&f.path)
                && match range {
                    None => true,
                    Some((start, end)) => f.modified.is_some_and(|t| start <= t && t < end),
                };

            if wanted {
                total += 1;
                if matches.len() < limit {
                    matches.push(f);
                }
            }
        }

        SearchResult { matches, total }
    }
}
