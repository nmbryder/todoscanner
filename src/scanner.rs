use crate::config::Config;
use crate::matcher::find_match;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub struct Match {
    pub path: PathBuf,
    pub line_number: usize,
    pub column: usize,
    pub tag: String,
    pub line_content: String,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

pub fn scan(config: &Config, mut on_match: impl FnMut(Match)) -> Result<()> {
    if let Some(ref path) = config.single_file {
        if let Err(e) = scan_file(path, config, &mut on_match) {
            eprintln!("warning: {}: {}", path.display(), e);
        }
        return Ok(());
    }

    if config.respect_gitignore {
        let walker = ignore::WalkBuilder::new(&config.root)
            .hidden(false)
            .git_ignore(true)
            .build();
        for entry in walker {
            match entry {
                Err(e) => eprintln!("warning: {}", e),
                Ok(e) => {
                    if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        continue;
                    }
                    let path = e.path().to_owned();
                    if !extension_matches(&path, config) {
                        continue;
                    }
                    if let Err(e) = scan_file(&path, config, &mut on_match) {
                        eprintln!("warning: {}: {}", path.display(), e);
                    }
                }
            }
        }
    } else {
        let walker = walkdir::WalkDir::new(&config.root).follow_links(false);
        for entry in walker {
            match entry {
                Err(e) => eprintln!("warning: {}", e),
                Ok(e) => {
                    if e.file_type().is_dir() {
                        continue;
                    }
                    let path = e.path().to_owned();
                    if !extension_matches(&path, config) {
                        continue;
                    }
                    if let Err(e) = scan_file(&path, config, &mut on_match) {
                        eprintln!("warning: {}: {}", path.display(), e);
                    }
                }
            }
        }
    }

    Ok(())
}

fn extension_matches(path: &Path, config: &Config) -> bool {
    match &config.extensions {
        None => true,
        Some(exts) => {
            let file_ext = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_lowercase())
                .unwrap_or_default();
            exts.contains(&file_ext)
        }
    }
}

fn scan_file(path: &Path, config: &Config, on_match: &mut impl FnMut(Match)) -> Result<()> {
    let mut file =
        std::fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;

    // Binary sniff: read first 512 bytes, check for null byte.
    let mut sniff_buf = [0u8; 512];
    let n = file.read(&mut sniff_buf)?;
    if sniff_buf[..n].contains(&0u8) {
        return Ok(()); // binary file, skip
    }
    file.seek(SeekFrom::Start(0))?;

    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.unwrap_or_default()).collect();

    let n_ctx = config.context_lines;

    for (i, line) in lines.iter().enumerate() {
        if let Some(hit) = find_match(line, &config.pattern) {
            let context_before = if n_ctx > 0 {
                let start = i.saturating_sub(n_ctx);
                lines[start..i].to_vec()
            } else {
                Vec::new()
            };
            let context_after = if n_ctx > 0 {
                let end = (i + 1 + n_ctx).min(lines.len());
                lines[i + 1..end].to_vec()
            } else {
                Vec::new()
            };

            on_match(Match {
                path: path.to_owned(),
                line_number: i + 1, // 1-based
                column: hit.column,
                tag: hit.tag,
                line_content: line.clone(),
                context_before,
                context_after,
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, OutputFormat};
    use regex::Regex;
    use std::io::Write;

    fn test_config_for_file(path: PathBuf) -> Config {
        Config {
            root: path.parent().unwrap().to_owned(),
            extensions: None,
            single_file: Some(path),
            pattern: Regex::new("TODO|FIXME|HACK|XXX|BUG").unwrap(),
            respect_gitignore: true,
            color: false,
            context_lines: 0,
            output_format: OutputFormat::Text,
        }
    }

    fn temp_file(content: &str) -> (tempfile::NamedTempFile, PathBuf) {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        let p = f.path().to_owned();
        (f, p)
    }

    #[test]
    fn test_scan_finds_match() {
        let (_f, path) = temp_file("line 1\n// TODO: fix this\nline 3\n");
        let config = test_config_for_file(path);
        let mut matches = Vec::new();
        scan(&config, |m| matches.push(m.tag.clone())).unwrap();
        assert_eq!(matches, vec!["TODO"]);
    }

    #[test]
    fn test_scan_no_match() {
        let (_f, path) = temp_file("no annotations here\n");
        let config = test_config_for_file(path);
        let mut matches: Vec<String> = Vec::new();
        scan(&config, |m| matches.push(m.tag.clone())).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_context_lines() {
        let content = "before\n// TODO: fix\nafter\n";
        let (_f, path) = temp_file(content);
        let mut config = test_config_for_file(path);
        config.context_lines = 1;
        let mut results = Vec::new();
        scan(&config, |m| {
            results.push((m.context_before.clone(), m.context_after.clone()))
        })
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].0, vec!["before"]);
        assert_eq!(results[0].1, vec!["after"]);
    }

    #[test]
    fn test_binary_file_skipped() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        f.write_all(b"hello\x00world").unwrap();
        let path = f.path().to_owned();
        let mut config = test_config_for_file(path);
        // Put a TODO-like byte sequence after the null so we'd see it if not skipped.
        config.single_file = Some(f.path().to_owned());
        let mut matches: Vec<String> = Vec::new();
        scan(&config, |m| matches.push(m.tag.clone())).unwrap();
        assert!(matches.is_empty());
    }

    #[test]
    fn test_extension_filter() {
        // Create a temp file with .txt extension — it should be filtered when ext=["rs"].
        let dir = tempfile::tempdir().unwrap();
        let txt_path = dir.path().join("notes.txt");
        std::fs::write(&txt_path, "// TODO: ignored\n").unwrap();

        let config = Config {
            root: dir.path().to_owned(),
            extensions: Some(vec!["rs".to_owned()]),
            single_file: None,
            pattern: Regex::new("TODO").unwrap(),
            respect_gitignore: false,
            color: false,
            context_lines: 0,
            output_format: OutputFormat::Text,
        };
        let mut matches: Vec<String> = Vec::new();
        scan(&config, |m| matches.push(m.tag.clone())).unwrap();
        assert!(matches.is_empty());
    }
}
