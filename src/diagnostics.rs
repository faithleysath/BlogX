use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub path: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub message: String,
    pub snippet: Option<SourceSnippet>,
    pub help: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceSnippet {
    pub line: usize,
    pub column: usize,
    pub text: String,
    pub marker_len: usize,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Diagnostics {
    pub items: Vec<Diagnostic>,
}

impl Diagnostics {
    pub fn warn(&mut self, path: impl Into<Option<PathBuf>>, message: impl Into<String>) {
        self.items.push(Diagnostic {
            severity: Severity::Warning,
            path: path.into(),
            line: None,
            column: None,
            message: message.into(),
            snippet: None,
            help: None,
        });
    }

    pub fn warn_at(
        &mut self,
        path: impl Into<Option<PathBuf>>,
        line: usize,
        column: usize,
        message: impl Into<String>,
    ) {
        self.items.push(Diagnostic {
            severity: Severity::Warning,
            path: path.into(),
            line: Some(line),
            column: Some(column),
            message: message.into(),
            snippet: None,
            help: None,
        });
    }

    pub fn warn_at_with_context(
        &mut self,
        path: impl Into<Option<PathBuf>>,
        line: usize,
        column: usize,
        message: impl Into<String>,
        snippet: Option<SourceSnippet>,
        help: Option<String>,
    ) {
        self.items.push(Diagnostic {
            severity: Severity::Warning,
            path: path.into(),
            line: Some(line),
            column: Some(column),
            message: message.into(),
            snippet,
            help,
        });
    }

    pub fn warning_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| item.severity == Severity::Warning)
            .count()
    }

    pub fn has_errors(&self) -> bool {
        self.items
            .iter()
            .any(|item| item.severity == Severity::Error)
    }

    pub fn print(&self) {
        for item in &self.items {
            let label = match item.severity {
                Severity::Warning => "warning",
                Severity::Error => "error",
            };
            if let Some(path) = &item.path {
                if let (Some(line), Some(column)) = (item.line, item.column) {
                    eprintln!(
                        "{label}: {}:{line}:{column}: {}",
                        path.display(),
                        item.message
                    );
                } else {
                    eprintln!("{label}: {}: {}", path.display(), item.message);
                }
            } else {
                eprintln!("{label}: {}", item.message);
            }
            if let Some(snippet) = &item.snippet {
                if let Some(path) = &item.path {
                    eprintln!(
                        "  --> {}:{}:{}",
                        path.display(),
                        snippet.line,
                        snippet.column
                    );
                }
                let line_number = snippet.line.to_string();
                eprintln!("  {line_number} | {}", snippet.text);
                eprintln!(
                    "  {} | {}{}",
                    " ".repeat(line_number.len()),
                    " ".repeat(snippet.column.saturating_sub(1)),
                    "^".repeat(snippet.marker_len.max(1))
                );
            }
            if let Some(help) = &item.help {
                eprintln!("  help: {help}");
            }
        }
    }
}
