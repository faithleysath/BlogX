use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Instant,
};

use anyhow::{Context, Result, anyhow};
use comrak::{
    Arena, Options, format_html,
    nodes::{AstNode, NodeCodeBlock, NodeHtmlBlock, NodeValue},
    parse_document,
};
use katex::Opts;
use serde::{Deserialize, Serialize};
use syntect::{
    easy::HighlightLines,
    highlighting::ThemeSet,
    html::{IncludeBackground, styled_line_to_highlighted_html},
    parsing::SyntaxSet,
};

use crate::{
    config::{Config, UrlMode},
    diagnostics::{Diagnostics, SourceSnippet},
    paths::{is_markdown, page_public_url},
};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FrontMatter {
    pub title: Option<String>,
    pub layout: Option<String>,
    pub draft: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct RenderedMarkdown {
    pub front_matter: FrontMatter,
    pub html: String,
}

#[derive(Clone)]
pub struct MarkdownRenderer {
    config: Config,
    page_urls: Arc<HashMap<PathBuf, String>>,
    syntax_set: Arc<SyntaxSet>,
    theme_set: Arc<ThemeSet>,
    math_cache: Arc<Mutex<HashMap<String, String>>>,
    metrics: Arc<RenderMetrics>,
}

#[derive(Debug, Default)]
pub struct RenderMetrics {
    pub markdown_ms: AtomicU64,
    pub katex_ms: AtomicU64,
    pub code_highlight_ms: AtomicU64,
}

impl MarkdownRenderer {
    pub fn new(config: &Config, page_urls: HashMap<PathBuf, String>) -> Self {
        let math_cache = load_persistent_math_cache(config).unwrap_or_default();
        Self {
            config: config.clone(),
            page_urls: Arc::new(page_urls),
            syntax_set: Arc::new(SyntaxSet::load_defaults_newlines()),
            theme_set: Arc::new(ThemeSet::load_defaults()),
            math_cache: Arc::new(Mutex::new(math_cache)),
            metrics: Arc::new(RenderMetrics::default()),
        }
    }

    pub fn metrics_snapshot(&self) -> RenderMetricsSnapshot {
        RenderMetricsSnapshot {
            markdown_ms: self.metrics.markdown_ms.load(Ordering::Relaxed),
            katex_ms: self.metrics.katex_ms.load(Ordering::Relaxed),
            code_highlight_ms: self.metrics.code_highlight_ms.load(Ordering::Relaxed),
        }
    }

    pub fn write_persistent_cache(&self) -> Result<()> {
        let path = self.config.paths.cache.join("katex.json");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let cache = self.math_cache.lock().expect("math cache poisoned");
        fs::write(&path, serde_json::to_vec_pretty(&*cache)?)
            .with_context(|| format!("failed to write {}", path.display()))
    }

    pub fn render_inline_markdown(&self, raw: &str) -> Result<String> {
        let mut diagnostics = Diagnostics::default();
        let rendered = self.render(
            Path::new("<template>"),
            Path::new("template.md"),
            raw,
            &mut diagnostics,
        )?;
        Ok(rendered.html)
    }

    pub fn render(
        &self,
        source_path: &Path,
        relative_path: &Path,
        raw: &str,
        diagnostics: &mut Diagnostics,
    ) -> Result<RenderedMarkdown> {
        let render_start = Instant::now();
        let (front_matter, markdown, front_matter_warnings) = split_front_matter(raw)
            .map_err(|err| format_front_matter_error(source_path, relative_path, raw, err))?;
        for warning in front_matter_warnings {
            if let Some((line, column)) = warning.location {
                let snippet = SourceLocator::new(raw).snippet_at(line, column, warning.key.len());
                diagnostics.warn_at_with_context(
                    Some(relative_path.to_path_buf()),
                    line,
                    column,
                    warning.message,
                    snippet,
                    Some(format!(
                        "remove `{}` or model it in page content/templates; BlogX only supports title, layout, and draft",
                        warning.key
                    )),
                );
            } else {
                diagnostics.warn(Some(relative_path.to_path_buf()), warning.message);
            }
        }
        let locator = SourceLocator::new(markdown);
        warn_missing_image_alt(relative_path, markdown, &locator, diagnostics);
        let processed = preprocess_math(markdown, &self.config, &self.math_cache, &self.metrics)?;

        let arena = Arena::new();
        let mut options = Options::default();
        options.extension.table = true;
        options.extension.footnotes = true;
        options.extension.tasklist = true;
        options.extension.strikethrough = true;
        options.extension.header_ids = if self.config.markdown.heading_anchors {
            Some(String::new())
        } else {
            None
        };
        options.render.r#unsafe = self.config.markdown.unsafe_html;

        let root = parse_document(&arena, &processed, &options);
        self.rewrite_ast(root, relative_path, markdown, &locator, diagnostics)?;

        let mut html = String::new();
        format_html(root, &options, &mut html).context("failed to render markdown html")?;

        self.metrics
            .markdown_ms
            .fetch_add(render_start.elapsed().as_millis() as u64, Ordering::Relaxed);

        Ok(RenderedMarkdown { front_matter, html })
    }

    fn rewrite_ast<'a>(
        &self,
        root: &'a AstNode<'a>,
        current_relative_path: &Path,
        raw_markdown: &str,
        locator: &SourceLocator,
        diagnostics: &mut Diagnostics,
    ) -> Result<()> {
        for node in root.descendants() {
            let mut data = node.data.borrow_mut();
            match &mut data.value {
                NodeValue::Link(link) => {
                    if let Some(rewritten) = self.rewrite_markdown_link(
                        current_relative_path,
                        &link.url,
                        raw_markdown,
                        locator,
                        diagnostics,
                    ) {
                        link.url = rewritten;
                    }
                }
                NodeValue::Image(_) => {}
                NodeValue::CodeBlock(block) => {
                    let html = self.render_code_block(block)?;
                    data.value = NodeValue::HtmlBlock(NodeHtmlBlock {
                        block_type: 0,
                        literal: html,
                    });
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn rewrite_markdown_link(
        &self,
        current_relative_path: &Path,
        raw_url: &str,
        raw_markdown: &str,
        locator: &SourceLocator,
        diagnostics: &mut Diagnostics,
    ) -> Option<String> {
        if is_non_local_link(raw_url) {
            return None;
        }

        let (path_part, suffix) = split_link_suffix(raw_url);
        let target = Path::new(path_part);
        if !is_markdown(target) {
            return None;
        }

        let base = current_relative_path
            .parent()
            .unwrap_or_else(|| Path::new(""));
        let normalized = normalize_relative_path(&base.join(target));
        let target_url = if let Some(url) = self.page_urls.get(&normalized) {
            url.clone()
        } else {
            let source_pos = locator.find(raw_url).or_else(|| {
                find_in_source(raw_markdown, path_part).map(|idx| locator.line_col(idx))
            });
            if let Some((line, column)) = source_pos {
                let snippet = locator.snippet_at(line, column, raw_url.len());
                diagnostics.warn_at_with_context(
                    Some(current_relative_path.to_path_buf()),
                    line,
                    column,
                    format!("local markdown link target does not exist: {path_part}"),
                    snippet,
                    Some(
                        "create the target Markdown file or update the link to an existing page"
                            .to_string(),
                    ),
                );
            } else {
                diagnostics.warn(
                    Some(current_relative_path.to_path_buf()),
                    format!("local markdown link target does not exist: {path_part}"),
                );
            }
            return page_public_url(&normalized, self.config.theme.url_mode).ok();
        };
        Some(format!("{target_url}{suffix}"))
    }

    fn render_code_block(&self, block: &NodeCodeBlock) -> Result<String> {
        let lang = block.info.split_whitespace().next().unwrap_or_default();
        let syntax = self
            .syntax_set
            .find_syntax_by_token(lang)
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());
        let theme = self
            .theme_set
            .themes
            .get(&self.config.code.theme)
            .or_else(|| self.theme_set.themes.values().next())
            .ok_or_else(|| anyhow!("no syntax highlighting themes available"))?;

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut html = String::new();
        let highlight_start = Instant::now();
        let lang_class = if lang.is_empty() {
            String::new()
        } else {
            format!(
                " language-{}",
                html_escape::encode_double_quoted_attribute(lang)
            )
        };
        html.push_str(&format!("<pre><code class=\"blogx-code{lang_class}\">"));
        for (idx, line) in block.literal.lines().enumerate() {
            let ranges = highlighter
                .highlight_line(line, &self.syntax_set)
                .context("failed to highlight code line")?;
            let highlighted = styled_line_to_highlighted_html(&ranges, IncludeBackground::No)
                .map_err(|err| anyhow!("failed to convert highlighted code to html: {err:?}"))?;
            html.push_str("<span class=\"source-line\">");
            if self.config.code.line_numbers {
                html.push_str("<span class=\"line-number\" aria-hidden=\"true\">");
                html.push_str(&(idx + 1).to_string());
                html.push_str("</span>");
            }
            html.push_str(&highlighted);
            html.push_str("</span>\n");
        }
        html.push_str("</code></pre>");
        self.metrics.code_highlight_ms.fetch_add(
            highlight_start.elapsed().as_millis() as u64,
            Ordering::Relaxed,
        );
        Ok(html)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RenderMetricsSnapshot {
    pub markdown_ms: u64,
    pub katex_ms: u64,
    pub code_highlight_ms: u64,
}

fn load_persistent_math_cache(config: &Config) -> Result<HashMap<String, String>> {
    let path = config.paths.cache.join("katex.json");
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

struct SourceLocator {
    raw: String,
    line_starts: Vec<usize>,
}

impl SourceLocator {
    fn new(raw: &str) -> Self {
        let mut line_starts = vec![0];
        for (index, ch) in raw.char_indices() {
            if ch == '\n' {
                line_starts.push(index + 1);
            }
        }
        Self {
            raw: raw.to_string(),
            line_starts,
        }
    }

    fn find(&self, needle: &str) -> Option<(usize, usize)> {
        find_in_source(&self.raw, needle).map(|index| self.line_col(index))
    }

    fn line_col(&self, byte_index: usize) -> (usize, usize) {
        let line_index = match self.line_starts.binary_search(&byte_index) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_starts[line_index];
        let column = self.raw[line_start..byte_index].chars().count() + 1;
        (line_index + 1, column)
    }

    fn snippet_at(&self, line: usize, column: usize, marker_len: usize) -> Option<SourceSnippet> {
        let text = self.raw.lines().nth(line.checked_sub(1)?)?.to_string();
        Some(SourceSnippet {
            line,
            column,
            text,
            marker_len,
        })
    }
}

fn find_in_source(raw: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        None
    } else {
        raw.find(needle)
    }
}

fn warn_missing_image_alt(
    relative_path: &Path,
    raw_markdown: &str,
    locator: &SourceLocator,
    diagnostics: &mut Diagnostics,
) {
    let bytes = raw_markdown.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] != b'!' || bytes[index + 1] != b'[' || is_escaped_byte(bytes, index) {
            index += 1;
            continue;
        }

        let alt_start = index + 2;
        let Some(alt_end) = find_unescaped_byte(bytes, alt_start, b']') else {
            index += 2;
            continue;
        };
        if bytes.get(alt_end + 1) != Some(&b'(') {
            index = alt_end + 1;
            continue;
        }
        let alt_text = raw_markdown[alt_start..alt_end].trim();
        if alt_text.is_empty() {
            let (line, column) = locator.line_col(index);
            let snippet = locator.snippet_at(line, column, 2);
            diagnostics.warn_at_with_context(
                Some(relative_path.to_path_buf()),
                line,
                column,
                "image is missing alt text",
                snippet,
                Some(
                    "write descriptive text inside the image brackets, for example ![Diagram](image.png)"
                        .to_string(),
                ),
            );
        }
        index = alt_end + 1;
    }
}

fn find_unescaped_byte(bytes: &[u8], from: usize, needle: u8) -> Option<usize> {
    (from..bytes.len()).find(|&idx| bytes[idx] == needle && !is_escaped_byte(bytes, idx))
}

fn is_escaped_byte(bytes: &[u8], index: usize) -> bool {
    let mut count = 0;
    let mut current = index;
    while current > 0 && bytes[current - 1] == b'\\' {
        count += 1;
        current -= 1;
    }
    count % 2 == 1
}

#[derive(Debug, Clone)]
pub struct FrontMatterWarning {
    pub key: String,
    pub message: String,
    pub location: Option<(usize, usize)>,
}

pub fn split_front_matter(raw: &str) -> Result<(FrontMatter, &str, Vec<FrontMatterWarning>)> {
    let Some(rest) = raw.strip_prefix("---\n") else {
        return Ok((FrontMatter::default(), raw, Vec::new()));
    };
    let Some(end) = rest.find("\n---\n") else {
        return Ok((FrontMatter::default(), raw, Vec::new()));
    };
    let yaml = &rest[..end];
    let markdown = &rest[end + "\n---\n".len()..];
    let warnings = unknown_front_matter_warnings(yaml)?;
    let front_matter: FrontMatter = serde_yaml::from_str(yaml)?;
    Ok((front_matter, markdown, warnings))
}

pub(crate) fn format_front_matter_error(
    source_path: &Path,
    relative_path: &Path,
    raw: &str,
    err: anyhow::Error,
) -> anyhow::Error {
    let Some(yaml_err) = err.downcast_ref::<serde_yaml::Error>() else {
        return err.context(format!(
            "failed to parse front matter in {}",
            source_path.display()
        ));
    };
    let Some(location) = yaml_err.location() else {
        return anyhow!(
            "failed to parse front matter in {}: {yaml_err}",
            source_path.display()
        );
    };
    let line = location.line() + 1;
    let column = location.column();
    let snippet = raw.lines().nth(line.saturating_sub(1)).unwrap_or("");
    let marker = " ".repeat(column.saturating_sub(1));
    anyhow!(
        "failed to parse front matter in {}:{line}:{column}: {yaml_err}\n  {line} | {snippet}\n  {} | {marker}^",
        relative_path.display(),
        " ".repeat(line.to_string().len())
    )
}

fn unknown_front_matter_warnings(yaml: &str) -> Result<Vec<FrontMatterWarning>> {
    let value: serde_yaml::Value = serde_yaml::from_str(yaml)?;
    let mut warnings = Vec::new();
    let serde_yaml::Value::Mapping(mapping) = value else {
        return Ok(warnings);
    };
    for key in mapping.keys() {
        let Some(key) = key.as_str() else {
            continue;
        };
        if !matches!(key, "title" | "layout" | "draft") {
            warnings.push(FrontMatterWarning {
                key: key.to_string(),
                message: format!("unknown front matter field: {key}"),
                location: find_front_matter_key(yaml, key),
            });
        }
    }
    Ok(warnings)
}

fn find_front_matter_key(yaml: &str, key: &str) -> Option<(usize, usize)> {
    for (idx, line) in yaml.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(key) && trimmed[key.len()..].trim_start().starts_with(':') {
            let column = line.len() - trimmed.len() + 1;
            return Some((idx + 2, column));
        }
    }
    None
}

fn preprocess_math(
    raw: &str,
    config: &Config,
    cache: &Arc<Mutex<HashMap<String, String>>>,
    metrics: &Arc<RenderMetrics>,
) -> Result<String> {
    let mut out = String::with_capacity(raw.len());
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    let mut at_line_start = true;
    while i < chars.len() {
        if at_line_start && starts_fenced_code(&chars, i) {
            let fence: String = chars[i..i + 3].iter().collect();
            out.push_str(&fence);
            i += 3;
            at_line_start = false;
            while i < chars.len() {
                if at_line_start && starts_with(&chars, i, &fence) {
                    out.push_str(&fence);
                    i += 3;
                    at_line_start = false;
                    break;
                }
                let ch = chars[i];
                out.push(ch);
                at_line_start = ch == '\n';
                i += 1;
            }
            continue;
        }
        if chars[i] == '`' && !is_escaped(&chars, i) {
            let ticks = count_repeated(&chars, i, '`');
            for _ in 0..ticks {
                out.push('`');
            }
            i += ticks;
            while i < chars.len() {
                if chars[i] == '`' && count_repeated(&chars, i, '`') >= ticks {
                    for _ in 0..ticks {
                        out.push('`');
                    }
                    i += ticks;
                    at_line_start = false;
                    break;
                }
                let ch = chars[i];
                out.push(ch);
                at_line_start = ch == '\n';
                i += 1;
            }
            continue;
        }
        if starts_with(&chars, i, "$$")
            && let Some(end) = find_delim(&chars, i + 2, "$$")
        {
            let expr: String = chars[i + 2..end].iter().collect();
            out.push_str(&render_math_cached(&expr, true, config, cache, metrics)?);
            i = end + 2;
            at_line_start = false;
            continue;
        }
        if starts_with(&chars, i, "\\[")
            && let Some(end) = find_delim(&chars, i + 2, "\\]")
        {
            let expr: String = chars[i + 2..end].iter().collect();
            out.push_str(&render_math_cached(&expr, true, config, cache, metrics)?);
            i = end + 2;
            at_line_start = false;
            continue;
        }
        if starts_with(&chars, i, "\\(")
            && let Some(end) = find_delim(&chars, i + 2, "\\)")
        {
            let expr: String = chars[i + 2..end].iter().collect();
            out.push_str(&render_math_cached(&expr, false, config, cache, metrics)?);
            i = end + 2;
            at_line_start = false;
            continue;
        }
        if chars[i] == '$'
            && !is_escaped(&chars, i)
            && let Some(end) = find_inline_dollar(&chars, i + 1)
        {
            let expr: String = chars[i + 1..end].iter().collect();
            out.push_str(&render_math_cached(&expr, false, config, cache, metrics)?);
            i = end + 1;
            at_line_start = false;
            continue;
        }
        let ch = chars[i];
        out.push(ch);
        at_line_start = ch == '\n';
        i += 1;
    }
    Ok(out)
}

fn starts_fenced_code(chars: &[char], index: usize) -> bool {
    starts_with(chars, index, "```") || starts_with(chars, index, "~~~")
}

fn count_repeated(chars: &[char], index: usize, needle: char) -> usize {
    let mut count = 0;
    while chars.get(index + count).is_some_and(|ch| *ch == needle) {
        count += 1;
    }
    count
}

fn render_math_cached(
    expr: &str,
    display: bool,
    config: &Config,
    cache: &Arc<Mutex<HashMap<String, String>>>,
    metrics: &Arc<RenderMetrics>,
) -> Result<String> {
    let key = format!(
        "{}:{}:{}:{}:{}:{expr}",
        env!("CARGO_PKG_VERSION"),
        if display { "display" } else { "inline" },
        config.katex.throw_on_error,
        config.katex.error_color,
        config.katex.trust,
    );
    if let Some(value) = cache.lock().expect("math cache poisoned").get(&key) {
        return Ok(value.clone());
    }
    let render_start = Instant::now();
    let opts = Opts::builder()
        .display_mode(display)
        .throw_on_error(config.katex.throw_on_error)
        .error_color(config.katex.error_color.clone())
        .trust(config.katex.trust)
        .build()
        .map_err(|err| anyhow!("invalid katex options: {err:?}"))?;
    let rendered = katex::render_with_opts(expr.trim(), opts)
        .map_err(|err| anyhow!("failed to render math expression: {err:?}"))?;
    cache
        .lock()
        .expect("math cache poisoned")
        .insert(key, rendered.clone());
    metrics
        .katex_ms
        .fetch_add(render_start.elapsed().as_millis() as u64, Ordering::Relaxed);
    Ok(rendered)
}

fn starts_with(chars: &[char], index: usize, needle: &str) -> bool {
    chars
        .get(index..index + needle.chars().count())
        .is_some_and(|slice| slice.iter().collect::<String>() == needle)
}

fn find_delim(chars: &[char], from: usize, delim: &str) -> Option<usize> {
    (from..chars.len()).find(|&idx| starts_with(chars, idx, delim) && !is_escaped(chars, idx))
}

fn find_inline_dollar(chars: &[char], from: usize) -> Option<usize> {
    (from..chars.len()).find(|&idx| chars[idx] == '$' && !is_escaped(chars, idx))
}

fn is_escaped(chars: &[char], index: usize) -> bool {
    let mut count = 0;
    let mut i = index;
    while i > 0 && chars[i - 1] == '\\' {
        count += 1;
        i -= 1;
    }
    count % 2 == 1
}

fn split_link_suffix(raw_url: &str) -> (&str, &str) {
    let hash = raw_url.find('#');
    let query = raw_url.find('?');
    let split_at = match (hash, query) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => raw_url.len(),
    };
    raw_url.split_at(split_at)
}

fn is_non_local_link(raw_url: &str) -> bool {
    raw_url.starts_with('#') || raw_url.starts_with('/') || has_uri_scheme(raw_url)
}

fn has_uri_scheme(raw_url: &str) -> bool {
    let Some(colon) = raw_url.find(':') else {
        return false;
    };
    let scheme = &raw_url[..colon];
    let mut chars = scheme.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_alphabetic()
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '-' | '.'))
}

fn normalize_relative_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::Normal(value) => out.push(value),
            _ => {}
        }
    }
    out
}

pub fn collect_page_urls(paths: &[PathBuf], mode: UrlMode) -> Result<HashMap<PathBuf, String>> {
    paths
        .iter()
        .map(|path| Ok((path.clone(), page_public_url(path, mode)?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_front_matter() {
        let (fm, body, warnings) =
            split_front_matter("---\ntitle: Hello\ndraft: true\n---\n# Hi").unwrap();
        assert_eq!(fm.title.as_deref(), Some("Hello"));
        assert_eq!(fm.draft, Some(true));
        assert_eq!(body, "# Hi");
        assert!(warnings.is_empty());
    }

    #[test]
    fn detects_non_local_links_by_uri_scheme() {
        assert!(is_non_local_link("https://example.com/page.md"));
        assert!(is_non_local_link("mailto:hello@example.com"));
        assert!(is_non_local_link("ftp://example.com/file.md"));
        assert!(is_non_local_link("tel:+15551234567"));
        assert!(is_non_local_link("data:text/plain,hello.md"));
        assert!(is_non_local_link("#section"));
        assert!(is_non_local_link("/absolute/page.md"));
        assert!(!is_non_local_link("notes/hello.md"));
        assert!(!is_non_local_link("./notes/hello.md"));
        assert!(!is_non_local_link("../notes/hello.md"));
    }
}
