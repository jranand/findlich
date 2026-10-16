use crate::query::{FileKind, FileQuery, Scope, TimeRange};

const FILLER: &[&str] = &[
    "a", "all", "and", "file", "files", "find", "for", "from", "in", "last", "me", "my", "of",
    "past", "previous", "show", "the", "this", "with",
];

fn kind_word(word: &str) -> Option<FileKind> {
    match word {
        "pdf" | "pdfs" => Some(FileKind::Pdf),
        "doc" | "docs" | "docx" => Some(FileKind::Docx),
        "md" | "markdown" => Some(FileKind::Markdown),
        "jpg" | "jpeg" | "png" | "webp" | "image" | "images" | "photos" | "pictures" => {
            Some(FileKind::Images)
        }
        "pptx" | "slides" | "presentation" | "presentations" => Some(FileKind::Pptx),
        _ => None,
    }
}

fn scope_word(word: &str) -> Option<Scope> {
    match word {
        "everywhere" | "anywhere" => Some(Scope::Everywhere),
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

    let mut kind = FileKind::Any;
    let mut time = TimeRange::Any;
    let mut scope = Scope::UserFolders;
    let mut name_terms = Vec::new();

    let mut number: Option<u32> = None;
    let mut prev = "";

    for &word in &words {
        let found_kind = kind_word(word);
        let found_scope = scope_word(word);
        let mut found_time = time_words(prev, word);

        // Rolling window: a number followed by days/weeks.
        if let Some(n) = number {
            match word {
                "day" | "days" => found_time = Some(TimeRange::PastDays(n)),
                "week" | "weeks" => found_time = Some(TimeRange::PastDays(n.saturating_mul(7))),
                _ => {}
            }
        }

        let this_number = word.parse::<u32>().ok();

        let used = found_kind.is_some()
            || found_scope.is_some()
            || found_time.is_some()
            || this_number.is_some()
            || FILLER.contains(&word);

        if let Some(k) = found_kind {
            kind = k;
        }
        if let Some(s) = found_scope {
            scope = s;
        }
        if let Some(t) = found_time {
            time = t;
        }
        if !used {
            name_terms.push(word.to_string());
        }

        number = this_number;
        prev = word;
    }

    FileQuery {
        kind,
        time,
        scope,
        name_terms,
    }
}
