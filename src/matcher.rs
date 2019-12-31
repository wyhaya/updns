use regex::{Error, Regex};
use std::fmt;

#[derive(Debug)]
pub struct Matcher(MatchMode);

#[derive(Debug)]
enum MatchMode {
    Static(String),
    Wildcard(WildcardMatch),
    Regex(Regex),
}

const REGEX_WORD: char = '~';
const WILDCARD: char = '*';

impl Matcher {
    pub fn new(raw: &str) -> Result<Self, Error> {
        // Use regex: ~^example\.com$
        if raw.starts_with(REGEX_WORD) {
            let reg = raw.replacen(REGEX_WORD, "", 1);
            let mode = MatchMode::Regex(Regex::new(&reg)?);
            return Ok(Matcher(mode));
        }

        // Use wildcard match: *.example.com
        let find = raw.chars().any(|c| c == WILDCARD);
        if find {
            let mode = MatchMode::Wildcard(WildcardMatch::new(raw));
            return Ok(Matcher(mode));
        }

        // Plain Text: example.com
        Ok(Matcher(MatchMode::Static(raw.to_string())))
    }

    pub fn is_match(&self, domain: &str) -> bool {
        match &self.0 {
            MatchMode::Static(raw) => raw == domain,
            MatchMode::Wildcard(raw) => raw.is_match(domain),
            MatchMode::Regex(raw) => raw.is_match(domain),
        }
    }
}

#[derive(Debug)]
struct WildcardMatch {
    chars: Vec<char>,
}

impl WildcardMatch {
    fn new(raw: &str) -> Self {
        let mut chars = Vec::with_capacity(raw.len());
        for c in raw.chars() {
            chars.push(c);
        }
        Self { chars }
    }

    fn is_match(&self, text: &str) -> bool {
        let mut chars = text.chars();
        let mut dot = false;

        for cur in &self.chars {
            match cur {
                '*' => {
                    match chars.next() {
                        Some(c) => {
                            if c == '.' {
                                return false;
                            }
                        }
                        None => return false,
                    }
                    while let Some(n) = chars.next() {
                        if n == '.' {
                            dot = true;
                            break;
                        }
                    }
                }
                word => {
                    if dot {
                        if word == &'.' {
                            dot = false;
                            continue;
                        } else {
                            return false;
                        }
                    }
                    match chars.next() {
                        Some(c) => {
                            if word != &c {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
            }
        }
        if dot {
            return false;
        }
        chars.next().is_none()
    }
}

impl fmt::Display for Matcher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            MatchMode::Static(raw) => write!(f, "{}", raw),
            MatchMode::Wildcard(raw) => {
                let mut s = String::new();
                for ch in raw.chars.clone() {
                    s.push(ch);
                }
                write!(f, "{}", s)
            }
            MatchMode::Regex(raw) => write!(f, "~{}", raw.as_str()),
        }
    }
}

#[cfg(test)]
mod test_matcher {
    use super::*;

    #[test]
    fn test_create() {}

    #[test]
    fn test_text() {
        let matcher = Matcher::new("example.com").unwrap();
        assert!(matcher.is_match("example.com"));
        assert!(!matcher.is_match("-example.com"));
        assert!(!matcher.is_match("example.com.cn"));
    }

    #[test]
    fn test_wildcard() {
        let matcher = Matcher::new("*").unwrap();
        assert!(matcher.is_match("localhost"));
        assert!(!matcher.is_match(".localhost"));
        assert!(!matcher.is_match("localhost."));
        assert!(!matcher.is_match("local.host"));

        let matcher = Matcher::new("*.com").unwrap();
        assert!(matcher.is_match("test.com"));
        assert!(matcher.is_match("example.com"));
        assert!(!matcher.is_match("test.test"));
        assert!(!matcher.is_match(".test.com"));
        assert!(!matcher.is_match("test.com."));
        assert!(!matcher.is_match("test.test.com"));

        let matcher = Matcher::new("*.*").unwrap();
        assert!(matcher.is_match("test.test"));
        assert!(!matcher.is_match(".test.test"));
        assert!(!matcher.is_match("test.test."));
        assert!(!matcher.is_match("test.test.test"));

        let matcher = Matcher::new("*.example.com").unwrap();
        assert!(matcher.is_match("test.example.com"));
        assert!(matcher.is_match("example.example.com"));
        assert!(!matcher.is_match("test.example.com.com"));
        assert!(!matcher.is_match("test.test.example.com"));

        let matcher = Matcher::new("*.example.*").unwrap();
        assert!(matcher.is_match("test.example.com"));
        assert!(matcher.is_match("example.example.com"));
        assert!(!matcher.is_match("test.test.example.test"));
        assert!(!matcher.is_match("test.example.test.test"));
    }

    #[test]
    fn test_regex() {
        let matcher = Matcher::new("~^example.com$").unwrap();
        assert!(matcher.is_match("example.com"));
        assert!(!matcher.is_match("test.example.com"));
    }

    #[test]
    fn test_to_string() {}
}
