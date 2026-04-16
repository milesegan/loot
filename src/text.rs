use regex::Regex;
use std::sync::OnceLock;

fn slash_regex() -> &'static Regex {
    static SLASH_REGEX: OnceLock<Regex> = OnceLock::new();
    SLASH_REGEX.get_or_init(|| Regex::new(r#"[/\\]+"#).expect("valid slash regex"))
}

fn remove_regex() -> &'static Regex {
    static REMOVE_REGEX: OnceLock<Regex> = OnceLock::new();
    REMOVE_REGEX.get_or_init(|| Regex::new(r#"["&,;.'(){}|:*!?#><-]"#).expect("valid remove regex"))
}

fn replace_regex() -> &'static Regex {
    static REPLACE_REGEX: OnceLock<Regex> = OnceLock::new();
    REPLACE_REGEX.get_or_init(|| Regex::new(r#"[ ~]+"#).expect("valid replace regex"))
}

/// Converts a free-form string into the normalized file-system-safe style used
/// by the normalization workflow.
pub fn tidy_string(value: &str) -> String {
    let tidied = value.trim();
    let no_diacritics = deunicode::deunicode_with_tofu(tidied, "_").to_lowercase();
    let escaped_slashes = slash_regex().replace_all(&no_diacritics, " ").into_owned();
    let removed = remove_regex()
        .replace_all(&escaped_slashes, "")
        .into_owned();

    replace_regex().replace_all(&removed, "_").into_owned()
}

/// Removes a leading `the_` prefix from normalized artist directory names.
pub fn strip_leading_the(value: &str) -> String {
    value.strip_prefix("the_").unwrap_or(value).to_owned()
}

#[cfg(test)]
mod tests {
    use super::{strip_leading_the, tidy_string};

    #[test]
    fn tidy_string_escapes_forward_slashes() {
        assert_eq!(tidy_string("AC/DC"), "ac_dc");
    }

    #[test]
    fn tidy_string_escapes_backward_slashes() {
        assert_eq!(tidy_string(r#"Live\Demo"#), "live_demo");
    }

    #[test]
    fn tidy_string_removes_diacritics_and_punctuation() {
        assert_eq!(tidy_string("  Björk: Debut?  "), "bjork_debut");
    }

    #[test]
    fn strip_leading_the_removes_normalized_article_prefix() {
        assert_eq!(strip_leading_the("the_beatles"), "beatles");
        assert_eq!(strip_leading_the("beatles"), "beatles");
    }
}
