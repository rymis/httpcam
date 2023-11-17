/// Basic regular expression implementation in clean Rust.
/// My implementation includes:
/// [a-z_] - symbol from set
/// [^a-z_] - symbol not from set
/// * - matches any number of any symbols
/// ? - matches one optional symbol
/// \ - escapes the next symbol
use std::error::Error;

enum Matcher {
    Literal(Vec<u8>),
    AnyChar,
    CharIn(Vec<u8>),
    CharNotIn(Vec<u8>),
    AnyString,
}

pub struct Pattern {
    pattern: Vec<Matcher>,
}

impl std::fmt::Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for m in &self.pattern {
            match m {
                Matcher::Literal(ref v) => {
                    write!(f, "{}", String::from_utf8_lossy(v))?;
                }
                Matcher::AnyChar => {
                    write!(f, "?")?;
                }
                Matcher::CharIn(ref v) => {
                    write!(f, "[{}]", String::from_utf8_lossy(v))?;
                }
                Matcher::CharNotIn(ref v) => {
                    write!(f, "[^{}]", String::from_utf8_lossy(v))?;
                }
                Matcher::AnyString => {
                    write!(f, "*")?;
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct ParseError {
    msg: String,
}

impl Error for ParseError {
    fn description(&self) -> &str {
        &self.msg
    }
}

impl ParseError {
    pub fn from(msg: &str) -> ParseError {
        ParseError {
            msg: String::from(msg),
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Parse error: {}", self.msg)
    }
}

/*
impl<E> From<E> for ParseError where E : Box<dyn Error> {
    fn from(value: E) -> ParseError {
        ParseError{msg: value.to_string()}
    }
}
*/

type Result<T> = std::result::Result<T, ParseError>;

pub struct CheckResult {
    groups: Vec<String>,
}

impl CheckResult {
    pub fn group_count(&self) -> usize {
        self.groups.len()
    }

    pub fn group(&self, idx: usize) -> &String {
        &self.groups[self.groups.len() - 1 - idx]
    }
}

impl std::fmt::Display for CheckResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "M[")?;
        for i in 0..self.group_count() {
            write!(f, " '{}'", self.group(i))?;
        }
        write!(f, " ]")
    }
}

impl Pattern {
    pub fn new(rx: &str) -> Result<Pattern> {
        let mut matchers: Vec<Matcher> = vec![];
        let mut i = 0;
        while i < rx.len() {
            let (m, next) = parse_matcher(rx, i)?;
            i = next;
            matchers.push(m);
        }

        Ok(Pattern { pattern: matchers })
    }

    pub fn check(&self, s: &str) -> Option<CheckResult> {
        let input = s.as_bytes();

        self.check_impl(input, 0, 0)
    }

    fn check_impl(&self, s: &[u8], pos: usize, rule_idx: usize) -> Option<CheckResult> {
        if rule_idx >= self.pattern.len() {
            return if pos == s.len() {
                Some(CheckResult { groups: vec![] })
            } else {
                None
            };
        }

        match self.pattern[rule_idx] {
            Matcher::Literal(ref v) => {
                if pos + v.len() <= s.len() && s[pos..pos + v.len()] == *v {
                    return self.check_impl(s, pos + v.len(), rule_idx + 1);
                }
            }
            Matcher::AnyChar => {
                if pos < s.len() {
                    return self.check_one_char_impl(s, pos, rule_idx);
                }
            }
            Matcher::CharIn(ref v) => {
                if pos < s.len() && char_in_impl(v, s[pos]) {
                    return self.check_one_char_impl(s, pos, rule_idx);
                }
            }
            Matcher::CharNotIn(ref v) => {
                if pos < s.len() && !char_in_impl(v, s[pos]) {
                    return self.check_one_char_impl(s, pos, rule_idx);
                }
            }
            Matcher::AnyString => {
                let mut p = pos;
                while p < s.len() {
                    let mut m = self.check_impl(s, p, rule_idx + 1);
                    match m {
                        Some(mut m) => {
                            m.groups
                                .push(String::from_utf8_lossy(&s[pos..p]).to_string());
                            return Some(m);
                        }
                        None => (),
                    }
                    p = p + 1;
                }
            }
        }

        None
    }

    fn check_one_char_impl(&self, s: &[u8], pos: usize, rule_idx: usize) -> Option<CheckResult> {
        let tail = self.check_impl(s, pos + 1, rule_idx + 1);
        match tail {
            Some(mut tail) => {
                tail.groups
                    .push(String::from_utf8_lossy(&s[pos..pos + 1]).to_string());
                Some(tail)
            }
            None => None,
        }
    }
}

fn char_in_impl(set: &[u8], c: u8) -> bool {
    let mut l = 0;
    let mut r = set.len();

    if set.len() == 0 {
        return false;
    }

    while l + 1 < r {
        let m = l + (r - l) / 2;
        if set[m] == c {
            return true;
        }
        if set[m] < c {
            l = m;
        } else {
            r = m;
        }
    }

    return set[l] == c;
}

const STAR: u8 = '*' as u8;
const QMARK: u8 = '?' as u8;
const BACKSLASH: u8 = '\\' as u8;
const OPENSQBRACKET: u8 = '[' as u8;
const CLOSESQBRACKET: u8 = ']' as u8;
const MINUS: u8 = '-' as u8;
const INVERT: u8 = '^' as u8;

fn check_index(re: &[u8], idx: usize) -> Result<()> {
    if idx >= re.len() {
        return Err(ParseError::from("Unexpected end of string"));
    }

    Ok(())
}

fn parse_matcher(rx: &str, idx: usize) -> Result<(Matcher, usize)> {
    let re = rx.as_bytes();

    if re[idx] == STAR {
        Ok((Matcher::AnyString, idx + 1))
    } else if re[idx] == QMARK {
        Ok((Matcher::AnyChar, idx + 1))
    } else if re[idx] == BACKSLASH {
        Ok((Matcher::Literal(re[idx + 1..idx + 2].to_vec()), idx + 2))
    } else if re[idx] == OPENSQBRACKET {
        let mut i = idx + 1;
        let mut invert = false;
        let mut set: Vec<u8> = vec![];

        check_index(re, i)?;

        if re[i] == INVERT {
            i += 1;
            invert = true;
        }

        while i < re.len() && re[i] != CLOSESQBRACKET {
            let (begin, end, next) = parse_set_item(re, i)?;
            i = next;
            for c in begin..end + 1 {
                set.push(c);
            }
        }

        check_index(re, i)?;

        set.sort();
        let mut fset: Vec<u8> = vec![];

        let mut prev: u8 = 0;
        for c in set {
            if c != prev {
                fset.push(c);
                prev = c;
            }
        }

        if invert {
            Ok((Matcher::CharNotIn(fset), i + 1))
        } else {
            Ok((Matcher::CharIn(fset), i + 1))
        }
    } else {
        let start = idx;
        let mut end = idx + 1;
        while end < re.len() && !is_special(re[end]) {
            end += 1;
        }
        Ok((Matcher::Literal(re[start..end].to_vec()), end))
    }
}

fn parse_set_item(rx: &[u8], idx: usize) -> Result<(u8, u8, usize)> {
    let mut i = idx;
    let mut begin = rx[i];
    if begin == BACKSLASH {
        i += 1;
        check_index(rx, i)?;
        begin = rx[i];
    }

    i += 1;
    check_index(rx, i)?;
    if rx[i] != MINUS {
        return Ok((begin, begin, i));
    }

    // Next char is minus:
    check_index(rx, i + 1)?;
    if rx[i + 1] == CLOSESQBRACKET {
        // Last minus is a char
        return Ok((begin, begin, i));
    }
    i = i + 1;
    let end = if rx[i] == BACKSLASH {
        i = i + 1;
        check_index(rx, i)?;
        rx[i + 1]
    } else {
        rx[i]
    };

    Ok((begin, end, i + 1))
}

fn is_special(c: u8) -> bool {
    c == STAR || c == QMARK || c == OPENSQBRACKET || c == BACKSLASH
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    fn compile_rx(r: &str, ok: bool) -> bool {
        let rx = Pattern::new(r);
        match rx {
            Ok(_) => ok,
            Err(_) => !ok,
        }
    }

    fn match_rx(rx: &str, s: &str, ok: bool) -> bool {
        let p = Pattern::new(rx);
        match p {
            Ok(p) => {
                let m = p.check(s);
                match m {
                    Some(x) => ok,
                    None => !ok,
                }
            }
            Err(_) => false,
        }
    }

    fn match_groups(rx: &str, s: &str, groups: &[&str]) -> bool {
        let p = Pattern::new(rx).unwrap();
        let m = p.check(s).unwrap();

        if groups.len() != m.group_count() {
            println!("GROUP LEN: {} {}", groups.len(), m.group_count());
            return false;
        }

        for i in 0..groups.len() {
            if groups[i] != m.group(i) {
                println!("GROUP[{}]: {} {}", i, groups[i], m.group(i));
                return false;
            }
        }

        true
    }

    #[test]
    fn test_compile() {
        assert!(compile_rx("", true));
        assert!(compile_rx("asd*def.xxx", true));
        assert!(compile_rx("asd*[1-2].xxx", true));
        assert!(compile_rx("asd*\\[1-2.xxx", true));
        assert!(compile_rx("asd*[1-2.xxx", false));
    }

    #[test]
    fn test_match() {
        assert!(match_rx("ab", "ab", true));
        assert!(match_rx("a*b", "ab", true));
        assert!(match_rx("a*b", "acb", true));
        assert!(match_rx("a*b", "accccccccccb", true));

        assert!(match_rx("a[c-e]b", "acb", true));
        assert!(match_rx("a[c-e]b", "adb", true));
        assert!(match_rx("a[c-e]b", "aeb", true));
        assert!(match_rx("a[c-e]b", "aab", false));
        assert!(match_rx("a[c-e]b", "afb", false));

        assert!(match_rx("a?b", "a$b", true));

        assert!(match_rx("a*b?x", "acccb$x", true));
    }

    #[test]
    fn test_match_group() {
        assert!(match_groups("a*b", "acccb", &["ccc"]));
        assert!(match_groups("a*b?x", "acccb$x", &["ccc", "$"]));
        assert!(match_groups("a*b[0-9]x", "acccb4x", &["ccc", "4"]));
    }

    #[test]
    fn test_char_in() {
        assert!(!char_in_impl(&[], 3));
        assert!(!char_in_impl(&[1], 0));
        assert!(char_in_impl(&[1], 1));
        assert!(!char_in_impl(&[1], 2));
        assert!(!char_in_impl(&[1, 2, 3], 0));
        assert!(char_in_impl(&[1, 2, 3], 1));
        assert!(char_in_impl(&[1, 2, 3], 2));
        assert!(char_in_impl(&[1, 2, 3], 3));
        assert!(!char_in_impl(&[1, 2, 3], 4));
    }
}
