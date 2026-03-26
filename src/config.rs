use anyhow::{anyhow, Context, Result};
use clap::Parser;
use regex::Regex;
use std::path::PathBuf;
use std::str::FromStr;

const DEFAULT_PATTERN: &str = "TODO|FIXME|HACK|XXX|BUG";

#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Text,
    Json,
    Csv,
}

impl FromStr for OutputFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "text" => Ok(OutputFormat::Text),
            "json" => Ok(OutputFormat::Json),
            "csv" => Ok(OutputFormat::Csv),
            other => Err(anyhow!(
                "unknown output format '{}'; expected text, json, or csv",
                other
            )),
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "todoscan",
    version,
    about = "Scan source files for annotation tags (TODO, FIXME, HACK, XXX, BUG)"
)]
pub struct Args {
    /// Directory or file to scan (default: current directory)
    pub path: Option<PathBuf>,

    /// Comma-separated file extensions to include (e.g. rs,py,js)
    #[arg(short = 'e', long = "ext")]
    pub ext: Option<String>,

    /// Scan a single specific file
    #[arg(short = 'f', long = "file")]
    pub file: Option<PathBuf>,

    /// Regex pattern to match (default: "TODO|FIXME|HACK|XXX|BUG")
    #[arg(short = 'p', long = "pattern")]
    pub pattern: Option<String>,

    /// Case-insensitive matching
    #[arg(short = 'i', long = "ignore-case")]
    pub ignore_case: bool,

    /// Don't respect .gitignore rules
    #[arg(long = "no-gitignore")]
    pub no_gitignore: bool,

    /// Disable colored output
    #[arg(long = "no-color")]
    pub no_color: bool,

    /// Show N lines of context around each match (default: 0)
    #[arg(short = 'c', long = "context", default_value_t = 0)]
    pub context: usize,

    /// Output format: text (default), json, csv
    #[arg(short = 'o', long = "output", default_value = "text")]
    pub output: String,
}

pub struct Config {
    pub root: PathBuf,
    pub extensions: Option<Vec<String>>,
    pub single_file: Option<PathBuf>,
    pub pattern: Regex,
    pub respect_gitignore: bool,
    pub color: bool,
    pub context_lines: usize,
    pub output_format: OutputFormat,
}

impl Config {
    pub fn from_args(args: Args) -> Result<Self> {
        let output_format = OutputFormat::from_str(&args.output)?;

        let root = if let Some(p) = args.path {
            std::fs::canonicalize(&p)
                .with_context(|| format!("path does not exist: {}", p.display()))?
        } else {
            std::env::current_dir().context("failed to get current directory")?
        };

        let single_file = if let Some(f) = args.file {
            let canon = std::fs::canonicalize(&f)
                .with_context(|| format!("path does not exist: {}", f.display()))?;
            if !canon.is_file() {
                return Err(anyhow!("'{}' is not a file", f.display()));
            }
            Some(canon)
        } else {
            None
        };

        let extensions = args.ext.map(|e| {
            e.split(',')
                .map(|s| s.trim().trim_start_matches('.').to_lowercase())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        });

        let raw_pattern = args.pattern.as_deref().unwrap_or(DEFAULT_PATTERN);
        let final_pattern = if args.ignore_case {
            format!("(?i){}", raw_pattern)
        } else {
            raw_pattern.to_owned()
        };
        let pattern = Regex::new(&final_pattern)
            .with_context(|| format!("invalid regex pattern: {}", raw_pattern))?;

        Ok(Config {
            root,
            extensions,
            single_file,
            pattern,
            respect_gitignore: !args.no_gitignore,
            color: !args.no_color,
            context_lines: args.context,
            output_format,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_args(overrides: impl FnOnce(&mut Args)) -> Args {
        let mut args = Args {
            path: None,
            ext: None,
            file: None,
            pattern: None,
            ignore_case: false,
            no_gitignore: false,
            no_color: false,
            context: 0,
            output: "text".to_owned(),
        };
        overrides(&mut args);
        args
    }

    #[test]
    fn test_invalid_regex() {
        let args = make_args(|a| a.pattern = Some("[invalid".to_owned()));
        assert!(Config::from_args(args).is_err());
    }

    #[test]
    fn test_nonexistent_path() {
        let args = make_args(|a| a.path = Some(PathBuf::from("/nonexistent/path/xyz")));
        let result = Config::from_args(args);
        assert!(result.is_err());
        assert!(result.err().unwrap().to_string().contains("does not exist"));
    }

    #[test]
    fn test_extension_strip_dot() {
        let args = make_args(|a| a.ext = Some(".rs".to_owned()));
        let config = Config::from_args(args).unwrap();
        assert_eq!(config.extensions, Some(vec!["rs".to_owned()]));
    }

    #[test]
    fn test_extension_multiple() {
        let args = make_args(|a| a.ext = Some("rs,py,js".to_owned()));
        let config = Config::from_args(args).unwrap();
        assert_eq!(
            config.extensions,
            Some(vec!["rs".to_owned(), "py".to_owned(), "js".to_owned()])
        );
    }

    #[test]
    fn test_default_pattern_matches() {
        let args = make_args(|_| {});
        let config = Config::from_args(args).unwrap();
        assert!(config.pattern.is_match("// TODO: fix this"));
        assert!(config.pattern.is_match("// FIXME: broken"));
        assert!(config.pattern.is_match("# HACK: workaround"));
        assert!(config.pattern.is_match("/* XXX: unclear */"));
        assert!(config.pattern.is_match("// BUG: crash here"));
    }

    #[test]
    fn test_ignore_case_wraps_pattern() {
        let args = make_args(|a| {
            a.pattern = Some("todo".to_owned());
            a.ignore_case = true;
        });
        let config = Config::from_args(args).unwrap();
        assert!(config.pattern.is_match("// TODO: fix"));
        assert!(config.pattern.is_match("// todo: fix"));
    }

    #[test]
    fn test_output_format_invalid() {
        let args = make_args(|a| a.output = "xml".to_owned());
        assert!(Config::from_args(args).is_err());
    }

    #[test]
    fn test_respect_gitignore_default() {
        let args = make_args(|_| {});
        let config = Config::from_args(args).unwrap();
        assert!(config.respect_gitignore);
    }

    #[test]
    fn test_no_gitignore_flag() {
        let args = make_args(|a| a.no_gitignore = true);
        let config = Config::from_args(args).unwrap();
        assert!(!config.respect_gitignore);
    }
}
