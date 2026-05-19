use std::{fs, path::PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub site: SiteConfig,
    pub paths: PathConfig,
    pub theme: ThemeConfig,
    pub markdown: MarkdownConfig,
    pub katex: KatexConfig,
    pub code: CodeConfig,
    pub assets: AssetConfig,
    pub serve: ServeConfig,
    pub site_infra: SiteInfraConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SiteConfig {
    pub title: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PathConfig {
    pub content: PathBuf,
    pub output: PathBuf,
    pub theme: PathBuf,
    pub cache: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeConfig {
    pub default_layout: String,
    pub url_mode: UrlMode,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UrlMode {
    #[default]
    Html,
    Clean,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MarkdownConfig {
    pub heading_anchors: bool,
    pub unsafe_html: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KatexConfig {
    pub throw_on_error: bool,
    pub error_color: String,
    pub trust: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CodeConfig {
    pub theme: String,
    pub line_numbers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AssetConfig {
    pub fingerprint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServeConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SiteInfraConfig {
    pub sitemap: bool,
    pub robots: bool,
}

impl Default for SiteConfig {
    fn default() -> Self {
        Self {
            title: "My Blog".to_string(),
            base_url: "/".to_string(),
        }
    }
}

impl Default for PathConfig {
    fn default() -> Self {
        Self {
            content: PathBuf::from("content"),
            output: PathBuf::from("public"),
            theme: PathBuf::from("theme"),
            cache: PathBuf::from(".blogx/cache"),
        }
    }
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self {
            default_layout: "page.html".to_string(),
            url_mode: UrlMode::Html,
        }
    }
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            heading_anchors: true,
            unsafe_html: true,
        }
    }
}

impl Default for KatexConfig {
    fn default() -> Self {
        Self {
            throw_on_error: true,
            error_color: "#cc0000".to_string(),
            trust: false,
        }
    }
}

impl Default for CodeConfig {
    fn default() -> Self {
        Self {
            theme: "base16-ocean.dark".to_string(),
            line_numbers: true,
        }
    }
}

impl Default for ServeConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8000,
        }
    }
}

impl Default for SiteInfraConfig {
    fn default() -> Self {
        Self {
            sitemap: true,
            robots: true,
        }
    }
}

impl Config {
    pub fn load_from_current_dir() -> Result<Self> {
        let path = PathBuf::from("blogx.toml");
        if !path.exists() {
            return Ok(Self::default());
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("failed to read config {}", path.display()))?;
        toml::from_str(&raw).map_err(|err| format_toml_error(&path, &raw, err))
    }
}

fn format_toml_error(path: &std::path::Path, raw: &str, err: toml::de::Error) -> anyhow::Error {
    let mut message = format!("failed to parse {}", path.display());
    if let Some(span) = err.span() {
        let (line, column) = line_column_for_offset(raw, span.start);
        message.push_str(&format!(":{line}:{column}"));
    }
    message.push_str(&format!(": {err}"));
    anyhow!(message)
}

fn line_column_for_offset(source: &str, byte_offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut line_start = 0;
    for (index, ch) in source.char_indices() {
        if index >= byte_offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            line_start = index + 1;
        }
    }
    let column = source[line_start..byte_offset.min(source.len())]
        .chars()
        .count()
        + 1;
    (line, column)
}
