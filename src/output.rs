use crate::config::{Config, OutputFormat};
use crate::scanner::Match;
use colored::Colorize;
use serde::Serialize;
use std::path::{Path, PathBuf};

pub struct Printer {
    format: OutputFormat,
    root: PathBuf,
    buffer: Vec<Match>,
}

impl Printer {
    pub fn new(config: &Config) -> Self {
        Printer {
            format: config.output_format.clone(),
            root: config.root.clone(),
            buffer: Vec::new(),
        }
    }

    pub fn print(&mut self, m: Match) {
        match self.format {
            OutputFormat::Text => print_text_match(&m, &self.root),
            OutputFormat::Json | OutputFormat::Csv => self.buffer.push(m),
        }
    }

    pub fn finish(self) {
        match self.format {
            OutputFormat::Text => {}
            OutputFormat::Json => print_json(&self.buffer),
            OutputFormat::Csv => print_csv(&self.buffer),
        }
    }
}

fn print_text_match(m: &Match, root: &Path) {
    let rel_path = m
        .path
        .strip_prefix(root)
        .unwrap_or(&m.path)
        .display()
        .to_string();

    let location = format!("{}:{}", rel_path, m.line_number);
    let line = m.line_content.trim();

    for ctx in &m.context_before {
        println!("  {}", ctx.trim().dimmed());
    }

    // Highlight the tag within the trimmed line.
    if let Some(tag_pos) = line.find(m.tag.as_str()) {
        let before = &line[..tag_pos];
        let tag_part = &line[tag_pos..tag_pos + m.tag.len()];
        let after = &line[tag_pos + m.tag.len()..];
        println!(
            "{}  {}{}{}",
            location.cyan().bold(),
            before,
            tag_part.red().bold(),
            after
        );
    } else {
        println!("{}  {}", location.cyan().bold(), line);
    }

    for ctx in &m.context_after {
        println!("  {}", ctx.trim().dimmed());
    }
}

#[derive(Serialize)]
struct JsonMatch {
    path: String,
    line_number: usize,
    column: usize,
    tag: String,
    line_content: String,
    context_before: Vec<String>,
    context_after: Vec<String>,
}

fn print_json(matches: &[Match]) {
    let json_matches: Vec<JsonMatch> = matches
        .iter()
        .map(|m| JsonMatch {
            path: m.path.display().to_string(),
            line_number: m.line_number,
            column: m.column,
            tag: m.tag.clone(),
            line_content: m.line_content.clone(),
            context_before: m.context_before.clone(),
            context_after: m.context_after.clone(),
        })
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&json_matches).unwrap_or_else(|_| "[]".to_owned())
    );
}

fn print_csv(matches: &[Match]) {
    println!("path,line_number,column,tag,line_content");
    for m in matches {
        println!(
            "{},{},{},{},{}",
            csv_escape(&m.path.display().to_string()),
            m.line_number,
            m.column,
            csv_escape(&m.tag),
            csv_escape(&m.line_content),
        );
    }
}

/// Escape a CSV field: wrap in double-quotes if the value contains a comma,
/// double-quote, or newline; double any embedded double-quotes.
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_escape_plain() {
        assert_eq!(csv_escape("hello"), "hello");
    }

    #[test]
    fn test_csv_escape_comma() {
        assert_eq!(csv_escape("foo(a, b)"), "\"foo(a, b)\"");
    }

    #[test]
    fn test_csv_escape_quotes() {
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn test_csv_escape_newline() {
        assert_eq!(csv_escape("line1\nline2"), "\"line1\nline2\"");
    }
}
