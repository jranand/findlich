use crate::query::{FileKind, FileQuery, Scope, TimeRange};

pub fn parse(input: &str) -> FileQuery {
    let text = input.to_lowercase();
    // Split on anything that isn't a letter or digit, so "pdfs, yesterday?" works.
    let words: Vec<&str> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();

    let mut kind = FileKind::Any;
    let mut time = TimeRange::Any;
    let mut scope = Scope::UserFolders;
    let mut number: Option<u32> = None;
    let mut prev = "";

    for &word in &words {
        // File type and scope: one word is enough.
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

        // Time: specific pairs first, general words last.
        match (prev, word) {
            (_, "today") => time = TimeRange::Today,
            (_, "yesterday") => time = TimeRange::Yesterday,
            (_, "recent") => time = TimeRange::PastDays(7),
            ("last" | "previous", "week") => time = TimeRange::LastWeek,
            ("last" | "previous", "month") => time = TimeRange::LastMonth,
            ("past", "week") => time = TimeRange::PastDays(7),
            ("past", "month") => time = TimeRange::PastDays(30),
            (_, "week") => time = TimeRange::ThisWeek,
            (_, "month") => time = TimeRange::ThisMonth,
            _ => {}
        }

        // Rolling window: a number followed by days/weeks overrides the above.
        if let Some(n) = number {
            match word {
                "day" | "days" => time = TimeRange::PastDays(n),
                "week" | "weeks" => time = TimeRange::PastDays(n.saturating_mul(7)),
                _ => {}
            }
        }

        number = word.parse::<u32>().ok();
        prev = word;
    }

    FileQuery { kind, time, scope }
}
