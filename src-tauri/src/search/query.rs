//! Parses search box text into a [`Filter`]:
//! `ext:iso size:>300mb age:>1y in:downloads kind:video "two words" name`.

use crate::categorize::Category;
use std::fmt;

const HOUR: i64 = 3600;
const DAY: i64 = 24 * HOUR;

/// What a search matches. All set conditions must hold.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Filter {
    /// Lowercase substrings that must all appear in the name.
    pub words: Vec<String>,
    /// Lowercase extensions without the dot; any may match.
    pub exts: Vec<String>,
    /// Inclusive bounds on allocated size.
    pub min_alloc: Option<u64>,
    pub max_alloc: Option<u64>,
    /// Last modified more than this many seconds ago.
    pub older_than: Option<i64>,
    /// Last modified less than this many seconds ago.
    pub newer_than: Option<i64>,
    /// `in:` values as typed: a place key, a path or a folder name; any may match.
    pub within: Vec<String>,
    pub kind: Option<Category>,
    /// Never return folders (Biggest).
    pub files_only: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// The offending term, as typed (without quotes).
    pub term: String,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.term, self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(query: &str) -> Result<Filter, ParseError> {
    let mut filter = Filter::default();
    for (term, quoted) in tokenize(query) {
        let fail = |message: &str| ParseError { term: term.clone(), message: message.to_string() };
        let filter_term = if quoted { None } else { term.split_once(':') };
        let Some((key, value)) = filter_term.filter(|(k, _)| k.len() > 1) else {
            filter.words.push(term.to_lowercase());
            continue;
        };
        if value.is_empty() {
            return Err(fail("needs a value"));
        }
        match key.to_ascii_lowercase().as_str() {
            "ext" => {
                let exts: Vec<String> = value
                    .split(',')
                    .map(|e| e.trim().trim_start_matches('.').to_lowercase())
                    .filter(|e| !e.is_empty())
                    .collect();
                if exts.is_empty() {
                    return Err(fail("needs an extension, like ext:iso"));
                }
                filter.exts.extend(exts);
            }
            "size" => {
                let (op, amount) = split_op(value);
                let bytes = parse_size(amount).ok_or_else(|| fail("use a size like 300mb or 1.5gb"))?;
                match op {
                    Op::Gt => filter.min_alloc = Some(bytes.saturating_add(1)),
                    Op::Ge | Op::None => filter.min_alloc = Some(bytes),
                    Op::Lt => filter.max_alloc = Some(bytes.saturating_sub(1)),
                    Op::Le => filter.max_alloc = Some(bytes),
                }
            }
            "age" => {
                let (op, amount) = split_op(value);
                let secs = parse_age(amount).ok_or_else(|| fail("use an age like 7d, 6m or 1y"))?;
                match op {
                    Op::Lt | Op::Le => filter.newer_than = Some(secs),
                    Op::Gt | Op::Ge | Op::None => filter.older_than = Some(secs),
                }
            }
            "in" => filter.within.push(value.to_string()),
            "kind" => {
                let kind = parse_kind(value).ok_or_else(|| {
                    fail("use video, images, audio, archives, code, documents, apps, data or other")
                })?;
                filter.kind = Some(kind);
            }
            _ => return Err(fail("unknown filter; use ext:, size:, age:, in: or kind:")),
        }
    }
    Ok(filter)
}

/// Splits on whitespace outside quotes and drops the quotes. The flag is true
/// for terms that start with a quote, which are always taken literally.
fn tokenize(query: &str) -> Vec<(String, bool)> {
    let mut terms = Vec::new();
    let mut cur = String::new();
    let mut quoted_start = false;
    let mut in_quote = false;
    let mut started = false;
    for c in query.chars() {
        match c {
            '"' => {
                if !started {
                    quoted_start = true;
                    started = true;
                }
                in_quote = !in_quote;
            }
            c if c.is_whitespace() && !in_quote => {
                if started && !cur.is_empty() {
                    terms.push((std::mem::take(&mut cur), quoted_start));
                }
                cur.clear();
                started = false;
                quoted_start = false;
            }
            c => {
                started = true;
                cur.push(c);
            }
        }
    }
    if !cur.is_empty() {
        terms.push((cur, quoted_start));
    }
    terms
}

enum Op {
    None,
    Gt,
    Ge,
    Lt,
    Le,
}

fn split_op(value: &str) -> (Op, &str) {
    if let Some(v) = value.strip_prefix(">=") {
        (Op::Ge, v)
    } else if let Some(v) = value.strip_prefix("<=") {
        (Op::Le, v)
    } else if let Some(v) = value.strip_prefix('>') {
        (Op::Gt, v)
    } else if let Some(v) = value.strip_prefix('<') {
        (Op::Lt, v)
    } else {
        (Op::None, value)
    }
}

/// Splits `1.5gb` into `(1.5, "gb")`.
fn number_and_unit(s: &str) -> Option<(f64, String)> {
    let s = s.trim();
    let split = s.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(s.len());
    let number: f64 = s[..split].parse().ok()?;
    (number.is_finite() && number >= 0.0).then(|| (number, s[split..].to_ascii_lowercase()))
}

fn parse_size(s: &str) -> Option<u64> {
    let (n, unit) = number_and_unit(s)?;
    let scale: u64 = match unit.as_str() {
        "" | "b" => 1,
        "k" | "kb" => 1 << 10,
        "m" | "mb" => 1 << 20,
        "g" | "gb" => 1 << 30,
        "t" | "tb" => 1 << 40,
        _ => return None,
    };
    Some((n * scale as f64).round() as u64)
}

fn parse_age(s: &str) -> Option<i64> {
    let (n, unit) = number_and_unit(s)?;
    let scale = match unit.as_str() {
        "h" => HOUR,
        "d" => DAY,
        "w" => 7 * DAY,
        "m" => 30 * DAY,
        "y" => 365 * DAY,
        _ => return None,
    };
    Some((n * scale as f64).round() as i64)
}

fn parse_kind(s: &str) -> Option<Category> {
    Some(match s.to_ascii_lowercase().as_str() {
        "video" | "videos" => Category::Video,
        "image" | "images" | "photos" => Category::Images,
        "audio" | "music" => Category::Audio,
        "archive" | "archives" => Category::Archives,
        "code" => Category::Code,
        "document" | "documents" | "docs" => Category::Documents,
        "app" | "apps" | "binaries" => Category::Apps,
        "data" | "caches" | "datacaches" => Category::DataCaches,
        "other" => Category::Other,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1 << 20;
    const GB: u64 = 1 << 30;
    const DAY: i64 = 86_400;

    fn ok(q: &str) -> Filter {
        parse(q).unwrap_or_else(|e| panic!("{q}: {e}"))
    }

    fn err(q: &str) -> ParseError {
        parse(q).expect_err(q)
    }

    #[test]
    fn bare_words_are_lowercased_name_terms() {
        let f = ok("  Holiday   MP4 ");
        assert_eq!(f.words, ["holiday", "mp4"]);
        assert_eq!(f, Filter { words: vec!["holiday".into(), "mp4".into()], ..Default::default() });
        assert_eq!(ok(""), Filter::default());
    }

    #[test]
    fn quotes_group_words_and_keep_colons_literal() {
        assert_eq!(ok(r#""report final" x"#).words, ["report final", "x"]);
        assert_eq!(ok(r#""ext:iso""#).words, ["ext:iso"]);
        assert_eq!(ok(r#"in:"Program Files""#).within, ["Program Files"]);
    }

    #[test]
    fn extensions_are_lists_without_dots() {
        assert_eq!(ok("ext:ISO").exts, ["iso"]);
        assert_eq!(ok("ext:.mp4,mkv ext:avi").exts, ["mp4", "mkv", "avi"]);
    }

    #[test]
    fn size_operators_and_units() {
        assert_eq!(ok("size:>300mb").min_alloc, Some(300 * MB + 1));
        assert_eq!(ok("size:>=300MB").min_alloc, Some(300 * MB));
        assert_eq!(ok("size:300mb").min_alloc, Some(300 * MB));
        assert_eq!(ok("size:<1kb").max_alloc, Some(1023));
        assert_eq!(ok("size:<=1k").max_alloc, Some(1024));
        assert_eq!(ok("size:>1.5gb").min_alloc, Some(GB + GB / 2 + 1));
        assert_eq!(ok("size:>2t").min_alloc, Some(2 * 1024 * GB + 1));
        assert_eq!(ok("size:<10").max_alloc, Some(9));
        let both = ok("size:>1mb size:<1gb");
        assert_eq!((both.min_alloc, both.max_alloc), (Some(MB + 1), Some(GB - 1)));
    }

    #[test]
    fn age_operators_and_units() {
        assert_eq!(ok("age:>1y").older_than, Some(365 * DAY));
        assert_eq!(ok("age:1y").older_than, Some(365 * DAY));
        assert_eq!(ok("age:<7d").newer_than, Some(7 * DAY));
        assert_eq!(ok("age:<12h").newer_than, Some(12 * 3600));
        assert_eq!(ok("age:>2w").older_than, Some(14 * DAY));
        assert_eq!(ok("age:>=6m").older_than, Some(180 * DAY));
    }

    #[test]
    fn in_and_kind() {
        assert_eq!(ok("in:downloads in:C:/temp").within, ["downloads", "C:/temp"]);
        assert_eq!(ok("kind:video").kind, Some(Category::Video));
        assert_eq!(ok("kind:Data").kind, Some(Category::DataCaches));
        assert_eq!(ok("kind:apps").kind, Some(Category::Apps));
    }

    #[test]
    fn mixed_query() {
        let f = ok("ext:iso size:>1gb age:>1y in:downloads win");
        assert_eq!(f.exts, ["iso"]);
        assert_eq!(f.min_alloc, Some(GB + 1));
        assert_eq!(f.older_than, Some(365 * DAY));
        assert_eq!(f.within, ["downloads"]);
        assert_eq!(f.words, ["win"]);
    }

    #[test]
    fn single_letter_prefix_is_a_word_not_a_filter() {
        // Looks like a drive path, e.g. pasted `C:\Users`.
        assert_eq!(ok(r"C:\Users").words, [r"c:\users"]);
    }

    #[test]
    fn bad_terms_are_errors_naming_the_term() {
        for (q, term) in [
            ("foo:bar", "foo:bar"),
            ("x size:>abc", "size:>abc"),
            ("size:>10zb", "size:>10zb"),
            ("age:3x", "age:3x"),
            ("age:>", "age:>"),
            ("ext:", "ext:"),
            ("in:", "in:"),
            ("kind:stuff", "kind:stuff"),
            ("ext:,", "ext:,"),
        ] {
            assert_eq!(err(q).term, term, "{q}");
        }
        assert!(err("foo:bar").to_string().contains("foo:bar"));
    }
}
