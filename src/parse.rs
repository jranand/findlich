use crate::query::{FileKind, FileQuery, Scope, TimeRange, UserFolder};

const FILLER: &[&str] = &[
    "a",
    "all",
    "an",
    "and",
    "any",
    "are",
    "at",
    "by",
    "called",
    "changed",
    "created",
    "document",
    "documents",
    "edited",
    "file",
    "files",
    "find",
    "folder",
    "folders",
    "for",
    "from",
    "in",
    "inside",
    "is",
    "last",
    "me",
    "modified",
    "my",
    "named",
    "of",
    "on",
    "opened",
    "or",
    "past",
    "previous",
    "saved",
    "show",
    "that",
    "the",
    "this",
    "to",
    "under",
    "was",
    "were",
    "which",
    "with",
];

fn kind_word(word: &str) -> Option<FileKind> {
    match word {
        // Categories: the whole group.
        "pdf" | "pdfs" => Some(FileKind::Pdf),
        "doc" | "docs" => Some(FileKind::Docx),
        "markdown" => Some(FileKind::Markdown),
        "image" | "images" | "photo" | "photos" | "picture" | "pictures" => Some(FileKind::Images),
        "audio" | "music" | "song" | "songs" | "track" | "tracks" => Some(FileKind::Audio),
        "video" | "videos" | "movie" | "movies" | "clip" | "clips" | "recording" | "recordings" => {
            Some(FileKind::Video)
        }
        "slides" | "presentation" | "presentations" => Some(FileKind::Pptx),
        // Exact extensions: just that type.
        "jpg" => Some(FileKind::Ext("jpg")),
        "jpeg" => Some(FileKind::Ext("jpeg")),
        "png" => Some(FileKind::Ext("png")),
        "webp" => Some(FileKind::Ext("webp")),
        "gif" => Some(FileKind::Ext("gif")),
        "svg" => Some(FileKind::Ext("svg")),
        "heic" => Some(FileKind::Ext("heic")),
        "docx" => Some(FileKind::Ext("docx")),
        "odt" => Some(FileKind::Ext("odt")),
        "pptx" => Some(FileKind::Ext("pptx")),
        "ppt" => Some(FileKind::Ext("ppt")),
        "odp" => Some(FileKind::Ext("odp")),
        "md" => Some(FileKind::Ext("md")),
        "mp3" => Some(FileKind::Ext("mp3")),
        "flac" => Some(FileKind::Ext("flac")),
        "wav" => Some(FileKind::Ext("wav")),
        "ogg" => Some(FileKind::Ext("ogg")),
        "opus" => Some(FileKind::Ext("opus")),
        "m4a" => Some(FileKind::Ext("m4a")),
        "mp4" => Some(FileKind::Ext("mp4")),
        "mkv" => Some(FileKind::Ext("mkv")),
        "webm" => Some(FileKind::Ext("webm")),
        "mov" => Some(FileKind::Ext("mov")),
        "avi" => Some(FileKind::Ext("avi")),
        _ => None,
    }
}

/// Folder words. Ambiguous ones only count after a place word ("in pictures").
fn folder_word(prev: &str, word: &str) -> Option<UserFolder> {
    let after_place_word = matches!(prev, "in" | "from" | "inside" | "under" | "on");
    match word {
        "desktop" => Some(UserFolder::Desktop),
        "download" | "downloads" => Some(UserFolder::Downloads),
        "documents" if after_place_word => Some(UserFolder::Documents),
        "pictures" if after_place_word => Some(UserFolder::Pictures),
        "music" if after_place_word => Some(UserFolder::Music),
        "videos" if after_place_word => Some(UserFolder::Videos),
        _ => None,
    }
}

fn time_words(prev: &str, word: &str) -> Option<TimeRange> {
    match (prev, word) {
        (_, "today") => Some(TimeRange::Today),
        (_, "yesterday") => Some(TimeRange::Yesterday),
        (_, "recent") => Some(TimeRange::PastDays(7)),
        ("last" | "previous", "week") => Some(TimeRange::LastWeek),
        ("last" | "previous", "month") => Some(TimeRange::LastMonth),
        ("past", "week") => Some(TimeRange::PastDays(7)),
        ("past", "month") => Some(TimeRange::PastDays(30)),
        (_, "week") => Some(TimeRange::ThisWeek),
        (_, "month") => Some(TimeRange::ThisMonth),
        _ => None,
    }
}

pub fn parse(input: &str) -> FileQuery {
    let text = input.to_lowercase();
    let words: Vec<&str> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();

    let mut kinds: Vec<FileKind> = Vec::new();
    let mut folders: Vec<UserFolder> = Vec::new();
    let mut everywhere = false;
    let mut time = TimeRange::Any;
    let mut name_terms = Vec::new();

    let mut number: Option<u32> = None;
    let mut prev = "";

    for (i, &word) in words.iter().enumerate() {
        let next = words.get(i + 1).copied().unwrap_or("");

        let found_folder = folder_word(prev, word);
        // A folder word is never also a file type: "pdf in pictures".
        let found_kind = if found_folder.is_some() {
            None
        } else {
            kind_word(word)
        };
        let is_everywhere = matches!(word, "everywhere" | "anywhere");
        let mut found_time = time_words(prev, word);

        // Rolling window: a number followed by a unit.
        if let Some(n) = number {
            let days = match word {
                "day" | "days" => Some(n),
                "week" | "weeks" => Some(n.saturating_mul(7)),
                "month" | "months" => Some(n.saturating_mul(30)),
                "year" | "years" => Some(n.saturating_mul(365)),
                _ => None,
            };
            if let Some(d) = days {
                found_time = Some(TimeRange::PastDays(d.min(36_500)));
            }
        }

        let this_number = word.parse::<u32>().ok();
        let is_count = this_number.is_some()
            && matches!(
                next,
                "day" | "days" | "week" | "weeks" | "month" | "months" | "year" | "years"
            );

        let used = found_kind.is_some()
            || found_folder.is_some()
            || is_everywhere
            || found_time.is_some()
            || is_count
            || FILLER.contains(&word);

        if let Some(k) = found_kind
            && !kinds.contains(&k)
        {
            kinds.push(k);
        }
        if let Some(f) = found_folder
            && !folders.contains(&f)
        {
            folders.push(f);
        }
        everywhere |= is_everywhere;
        if let Some(t) = found_time {
            time = t;
        }
        // Single letters (the "s" in "report's") are too vague to search for.
        if !used && word.chars().count() > 1 {
            name_terms.push(word.to_string());
        }

        number = this_number;
        prev = word;
    }

    let scope = if everywhere {
        Scope::Everywhere
    } else if !folders.is_empty() {
        Scope::Folders(folders)
    } else {
        Scope::UserFolders
    };

    FileQuery {
        kinds,
        time,
        scope,
        name_terms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{FileKind, Scope, TimeRange, UserFolder};

    #[test]
    fn pdf_last_week() {
        let q = parse("pdf last week");
        assert_eq!(q.kinds, vec![FileKind::Pdf]);
        assert_eq!(q.time, TimeRange::LastWeek);
        assert_eq!(q.scope, Scope::UserFolders);
        assert!(q.name_terms.is_empty());
    }
    #[test]
    fn photos_in_downloads() {
        let q = parse("photos in downloads");
        assert_eq!(q.kinds, vec![FileKind::Images]);
        assert_eq!(q.scope, Scope::Folders(vec![UserFolder::Downloads]));
    }

    #[test]
    fn pictures_alone_is_a_file_type() {
        let q = parse("pictures");
        assert_eq!(q.kinds, vec![FileKind::Images]);
        assert_eq!(q.scope, Scope::UserFolders);
    }

    #[test]
    fn in_pictures_is_a_folder() {
        let q = parse("in pictures");
        assert!(q.kinds.is_empty());
        assert_eq!(q.scope, Scope::Folders(vec![UserFolder::Pictures]));
    }

    #[test]
    fn jpg_is_an_exact_extension() {
        let q = parse("jpg");
        assert_eq!(q.kinds, vec![FileKind::Ext("jpg".into())]);
    }

    #[test]
    fn number_with_unit_is_rolling_window() {
        assert_eq!(parse("pdf 3 days").time, TimeRange::PastDays(3));
    }

    #[test]
    fn number_without_unit_is_a_name_word() {
        let q = parse("tax 2024");
        assert_eq!(q.name_terms, vec!["tax", "2024"]);
        assert_eq!(q.time, TimeRange::Any);
    }
}
