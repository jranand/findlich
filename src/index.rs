use crate::query::FileQuery;
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
/// A user's own folder called "build" stays searchable.
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

/// Why a folder should not be indexed, or None to index it.
fn skip_dir_reason(path: &Path, home: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;

    if DEPENDENCY_DIRS.contains(&name) {
        return Some("dependencies");
    }
    if let Ok(rel) = path.strip_prefix(home) {
        if TOOL_DIRS.iter().any(|d| rel == Path::new(d)) {
            return Some("tool data");
        }
    }
    // Standard marker: "this folder is regenerable cache".
    if path.join("CACHEDIR.TAG").exists() {
        return Some("cache");
    }
    // Every Python virtual environment contains this file.
    if path.join("pyvenv.cfg").exists() {
        return Some("python environment");
    }
    for (dir, markers) in BUILD_DIRS {
        if name == *dir {
            if let Some(parent) = path.parent() {
                if markers.iter().any(|m| parent.join(m).exists()) {
                    return Some("build output");
                }
            }
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
    /// Walk the home folder once and remember the files a person might look for.
    pub fn build() -> Index {
        let mut files = Vec::new();
        let Some(home) = dirs::home_dir() else {
            return Index {
                files,
                skipped: BTreeMap::new(),
            };
        };

        let mut walker = WalkBuilder::new(&home);
        walker
            .hidden(true) // skip dotfiles and dot-folders (~/.ssh, ~/.cache, ...)
            .git_ignore(true) // respect .gitignore inside projects
            .require_git(false) // ...even in folders that aren't git repos
            .git_global(false) // ignore personal git config: predictable for everyone
            .git_exclude(false)
            .parents(false) // never read ignore files above ~
            .add_custom_ignore_filename(".hidden"); // GNOME file-manager convention

        // The user's own rules, in .gitignore syntax.
        if let Some(config) = dirs::config_dir() {
            let user_rules = config.join("findlich/ignore");
            if user_rules.exists() {
                if let Some(err) = walker.add_ignore(&user_rules) {
                    eprintln!("findlich: problem reading {}: {err}", user_rules.display());
                }
            }
        }

        // Count what our own rules skip, so we can report it honestly.
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

        for entry in walker.build().filter_map(Result::ok) {
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

        // Newest first: every search keeps this order for free.
        files.sort_by(|a, b| b.modified.cmp(&a.modified));

        let skipped = std::mem::take(&mut *skipped.lock().unwrap());
        Index { files, skipped }
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Folders skipped by Findlich's own rules, counted by reason.
    pub fn skipped(&self) -> &BTreeMap<&'static str, usize> {
        &self.skipped
    }

    pub fn search(&self, q: &FileQuery) -> Vec<&IndexedFile> {
        let range = q.time.range().map(|(s, e)| (s.timestamp(), e.timestamp()));
        let roots = q.scope.roots();

        self.files
            .iter()
            .filter(|f| roots.iter().any(|r| f.path.starts_with(r)))
            .filter(|f| q.matches_kind(&f.path))
            .filter(|f| q.matches_name(&f.path))
            .filter(|f| match range {
                None => true,
                Some((start, end)) => f.modified.is_some_and(|t| start <= t && t < end),
            })
            .collect()
    }
}
