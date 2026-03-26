use regex::Regex;

pub struct LineMatch {
    pub column: usize, // 1-based
    pub tag: String,
}

/// Find the first annotation match in a line.
///
/// Note: The default pattern `TODO|FIXME|HACK|XXX|BUG` has no word boundaries,
/// so a line like `"NOTABUG"` will match `BUG`. This is intentional — users who
/// need word boundaries can supply a custom `--pattern` with `\b`.
pub fn find_match(line: &str, pattern: &Regex) -> Option<LineMatch> {
    pattern.find(line).map(|m| LineMatch {
        column: m.start() + 1,
        tag: m.as_str().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use regex::Regex;

    fn re(s: &str) -> Regex {
        Regex::new(s).unwrap()
    }

    #[test]
    fn test_basic_match() {
        let m = find_match("// TODO: fix this", &re("TODO|FIXME")).unwrap();
        assert_eq!(m.tag, "TODO");
        assert_eq!(m.column, 4);
    }

    #[test]
    fn test_no_match() {
        assert!(find_match("no annotation here", &re("TODO|FIXME")).is_none());
    }

    #[test]
    fn test_match_at_column_1() {
        let m = find_match("FIXME: urgent", &re("TODO|FIXME")).unwrap();
        assert_eq!(m.tag, "FIXME");
        assert_eq!(m.column, 1);
    }

    #[test]
    fn test_case_insensitive() {
        let m = find_match("// todo: fix", &re("(?i)TODO|FIXME")).unwrap();
        assert_eq!(m.tag, "todo");
    }

    #[test]
    fn test_whitespace_only_no_match() {
        assert!(find_match("   \t  ", &re("TODO|FIXME|HACK|XXX|BUG")).is_none());
    }

    #[test]
    fn test_bug_in_comment() {
        let m = find_match("# BUG in loop", &re("TODO|FIXME|HACK|XXX|BUG")).unwrap();
        assert_eq!(m.tag, "BUG");
    }

    #[test]
    fn test_no_word_boundary_notabug() {
        // Intentional: default pattern has no \b, so "NOTABUG" matches BUG.
        let m = find_match("// NOTABUG comment", &re("TODO|FIXME|HACK|XXX|BUG"));
        assert!(m.is_some());
        assert_eq!(m.unwrap().tag, "BUG");
    }

    #[test]
    fn test_first_match_returned() {
        // Line has both TODO and FIXME; only the first (TODO) is returned.
        let m = find_match("// TODO: also FIXME", &re("TODO|FIXME")).unwrap();
        assert_eq!(m.tag, "TODO");
    }
}
