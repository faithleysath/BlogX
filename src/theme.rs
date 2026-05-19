use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fmt, fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Instant,
};

use anyhow::{Context, Result, bail};
use minijinja::{
    Environment, Error as TemplateError, ErrorKind as TemplateErrorKind, Value, context,
    path_loader,
    value::{Enumerator, Object},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{config::Config, markdown::MarkdownRenderer};

#[derive(Debug, Clone, Serialize)]
pub struct SiteContext {
    pub title: String,
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageContext {
    pub title: Option<String>,
    pub source_path: String,
    pub output_path: String,
    pub url: String,
    pub layout: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildContext {
    pub version: String,
}

#[derive(Clone)]
pub struct Theme {
    root: PathBuf,
    metadata: ThemeMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemeMetadata {
    pub name: String,
    pub version: String,
}

impl Default for ThemeMetadata {
    fn default() -> Self {
        Self {
            name: "Unnamed Theme".to_string(),
            version: "0.0.0".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct AssetMap {
    entries: BTreeMap<String, String>,
}

#[derive(Clone)]
pub struct RenderPageInput {
    pub layout: String,
    pub site: SiteContext,
    pub page: PageContext,
    pub content: String,
    pub partials: BTreeMap<String, String>,
    pub partial_usage: PartialUsage,
    pub build: BuildContext,
    pub assets: AssetMap,
    pub page_urls: HashMap<String, String>,
    pub renderer: MarkdownRenderer,
}

#[derive(Debug, Clone)]
pub struct RenderedTemplate {
    pub html: String,
    pub layout_ms: u128,
}

impl Theme {
    pub fn load(root: &Path) -> Result<Self> {
        if !root.join("layouts").exists() {
            bail!("theme is missing layouts directory: {}", root.display());
        }
        let metadata_path = root.join("theme.toml");
        let metadata = if metadata_path.exists() {
            let raw = fs::read_to_string(&metadata_path)
                .with_context(|| format!("failed to read {}", metadata_path.display()))?;
            toml::from_str(&raw)
                .with_context(|| format!("failed to parse {}", metadata_path.display()))?
        } else {
            ThemeMetadata::default()
        };
        Ok(Self {
            root: root.to_path_buf(),
            metadata,
        })
    }

    pub fn render_page(&self, input: RenderPageInput) -> Result<RenderedTemplate> {
        let start = Instant::now();
        let mut env = self.environment();
        let asset_entries = input.assets.entries.clone();
        env.add_function("asset_url", move |path: String| -> Value {
            let trimmed = path.trim_start_matches('/');
            let url = asset_entries
                .get(trimmed)
                .cloned()
                .unwrap_or_else(|| format!("/assets/theme/{trimmed}"));
            Value::from_safe_string(url)
        });
        let page_urls = Arc::new(input.page_urls.clone());
        env.add_function("url_for", move |path: String| -> Value {
            let trimmed = path.trim_start_matches('/');
            let url = page_urls
                .get(trimmed)
                .cloned()
                .unwrap_or_else(|| path.clone());
            Value::from_safe_string(url)
        });
        let renderer = input.renderer.clone();
        env.add_function(
            "markdown",
            move |raw: String| -> Result<Value, TemplateError> {
                renderer
                    .render_inline_markdown(&raw)
                    .map(Value::from_safe_string)
                    .map_err(|err| {
                        TemplateError::new(
                            TemplateErrorKind::InvalidOperation,
                            format!("markdown helper failed: {err:#}"),
                        )
                    })
            },
        );

        let template_name = layout_template_name(&input.layout);
        let template = env.get_template(&template_name).map_err(|err| {
            format_template_error(&self.root, "failed to load layout", &input.layout, err)
        })?;

        let html = template
            .render(context! {
                site => input.site,
                page => input.page,
                content => Value::from_safe_string(input.content),
                partials => Value::from_object(TrackedPartials::new(input.partials, input.partial_usage)),
                assets => input.assets,
                build => input.build,
            })
            .map_err(|err| {
                format_template_error(&self.root, "failed to render layout", &input.layout, err)
            })?;

        Ok(RenderedTemplate {
            html,
            layout_ms: start.elapsed().as_millis(),
        })
    }

    pub fn assets_dir(&self) -> PathBuf {
        self.root.join("assets")
    }

    pub fn metadata(&self) -> &ThemeMetadata {
        &self.metadata
    }

    fn environment(&self) -> Environment<'static> {
        let mut env = Environment::new();
        env.set_loader(path_loader(self.root.clone()));
        env.set_debug(true);
        env
    }
}

#[derive(Debug, Clone, Default)]
pub struct PartialUsage {
    used: Arc<Mutex<HashSet<String>>>,
}

impl PartialUsage {
    pub fn used_keys(&self) -> HashSet<String> {
        self.used.lock().expect("partial usage poisoned").clone()
    }

    pub fn mark(&self, key: &str) {
        self.used
            .lock()
            .expect("partial usage poisoned")
            .insert(key.to_string());
    }
}

struct TrackedPartials {
    entries: BTreeMap<String, String>,
    usage: PartialUsage,
}

impl TrackedPartials {
    fn new(entries: BTreeMap<String, String>, usage: PartialUsage) -> Self {
        Self { entries, usage }
    }
}

impl fmt::Debug for TrackedPartials {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_map().entries(self.entries.iter()).finish()
    }
}

impl Object for TrackedPartials {
    fn get_value(self: &Arc<Self>, key: &Value) -> Option<Value> {
        self.get_value_by_str(key.as_str()?)
    }

    fn get_value_by_str(self: &Arc<Self>, key: &str) -> Option<Value> {
        let value = self.entries.get(key)?;
        self.usage.mark(key);
        Some(Value::from_safe_string(value.clone()))
    }

    fn enumerate(self: &Arc<Self>) -> Enumerator {
        Enumerator::Values(self.entries.keys().cloned().map(Value::from).collect())
    }

    fn enumerator_len(self: &Arc<Self>) -> Option<usize> {
        Some(self.entries.len())
    }
}

fn format_template_error(
    root: &Path,
    action: &str,
    layout: &str,
    err: TemplateError,
) -> anyhow::Error {
    let mut message = format!("{action} {layout}");
    if let Some(name) = err.name() {
        message.push_str(&format!(" at {}", root.join(name).display()));
    }
    let column = err
        .template_source()
        .zip(err.range())
        .map(|(source, range)| line_column_for_offset(source, range.start).1);
    if let Some(line) = err.line() {
        if let Some(column) = column {
            message.push_str(&format!(":{line}:{column}"));
        } else {
            message.push_str(&format!(":{line}"));
        }
    }
    message.push_str(&format!(": {err}"));
    if let Some(source) = err.template_source()
        && let Some(line) = err
            .line()
            .and_then(|line| source.lines().nth(line.saturating_sub(1)))
    {
        message.push_str(&format!("\n  {line}"));
        if let Some(column) = column {
            message.push_str(&format!("\n  {}^", " ".repeat(column.saturating_sub(1))));
        }
    }
    anyhow::anyhow!(message)
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

pub fn site_context(config: &Config) -> SiteContext {
    SiteContext {
        title: config.site.title.clone(),
        base_url: config.site.base_url.clone(),
    }
}

pub fn build_context() -> BuildContext {
    BuildContext {
        version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub fn load_theme_partials(
    content_partials: &BTreeMap<String, String>,
) -> BTreeMap<String, String> {
    content_partials.clone()
}

pub fn copy_theme_assets(theme: &Theme, output_dir: &Path) -> Result<usize> {
    copy_theme_assets_with_options(theme, output_dir, false).map(|result| result.count)
}

pub fn copy_theme_assets_with_options(
    theme: &Theme,
    output_dir: &Path,
    fingerprint: bool,
) -> Result<ThemeAssetCopy> {
    let assets_dir = theme.assets_dir();
    if !assets_dir.exists() {
        return Ok(ThemeAssetCopy::default());
    }
    copy_theme_assets_inner(&assets_dir, &output_dir.join("assets/theme"), fingerprint)
}

#[derive(Debug, Clone, Default)]
pub struct ThemeAssetCopy {
    pub count: usize,
    pub assets: AssetMap,
}

pub fn copy_dir_contents(source: &Path, dest: &Path) -> Result<usize> {
    let mut count = 0;
    for entry in walkdir::WalkDir::new(source) {
        let entry = entry.with_context(|| format!("failed to walk {}", source.display()))?;
        let path = entry.path();
        let relative = path.strip_prefix(source)?;
        let target = dest.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(&target)
                .with_context(|| format!("failed to create {}", target.display()))?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create {}", parent.display()))?;
            }
            fs::copy(path, &target).with_context(|| {
                format!("failed to copy {} to {}", path.display(), target.display())
            })?;
            count += 1;
        }
    }
    Ok(count)
}

fn layout_template_name(layout: &str) -> String {
    if layout.starts_with("layouts/") {
        layout.to_string()
    } else {
        format!("layouts/{layout}")
    }
}

fn copy_theme_assets_inner(
    source: &Path,
    dest: &Path,
    fingerprint: bool,
) -> Result<ThemeAssetCopy> {
    let mut files = Vec::new();

    for entry in walkdir::WalkDir::new(source) {
        let entry = entry.with_context(|| format!("failed to walk {}", source.display()))?;
        let path = entry.path();
        let relative = path.strip_prefix(source)?;
        if entry.file_type().is_dir() {
            fs::create_dir_all(dest.join(relative))
                .with_context(|| format!("failed to create {}", dest.join(relative).display()))?;
            continue;
        }

        let bytes =
            fs::read(path).with_context(|| format!("failed to read asset {}", path.display()))?;
        files.push(ThemeAssetFile {
            source_path: path.to_path_buf(),
            relative: relative.to_path_buf(),
            bytes,
        });
    }

    let mut output_by_original = BTreeMap::new();
    for file in &files {
        let output_relative = if fingerprint {
            fingerprinted_path(&file.relative, &file.bytes)
        } else {
            file.relative.clone()
        };
        output_by_original.insert(normalize_asset_path(&file.relative), output_relative);
    }

    if fingerprint {
        stabilize_css_fingerprints(&files, &mut output_by_original)?;
    }

    let mut count = 0;
    let mut entries = BTreeMap::new();
    let mut current_outputs = HashSet::new();
    for file in &files {
        let original_key = normalize_asset_path(&file.relative);
        let output_relative = output_by_original
            .get(&original_key)
            .cloned()
            .unwrap_or_else(|| file.relative.clone());
        let bytes = if fingerprint && is_css_asset(&file.relative) {
            rewrite_css_asset_urls(file, &output_relative, &output_by_original)?
        } else {
            file.bytes.clone()
        };
        let target = dest.join(&output_relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        if !target.exists() || fs::read(&target).unwrap_or_default() != bytes {
            fs::write(&target, &bytes)
                .with_context(|| format!("failed to write {}", target.display()))?;
        }
        entries.insert(
            original_key,
            format!("/assets/theme/{}", normalize_asset_path(&output_relative)),
        );
        current_outputs.insert(normalize_asset_path(&output_relative));
        count += 1;
    }

    remove_stale_theme_assets(dest, &current_outputs)?;

    Ok(ThemeAssetCopy {
        count,
        assets: AssetMap { entries },
    })
}

#[derive(Debug)]
struct ThemeAssetFile {
    source_path: PathBuf,
    relative: PathBuf,
    bytes: Vec<u8>,
}

fn stabilize_css_fingerprints(
    files: &[ThemeAssetFile],
    output_by_original: &mut BTreeMap<String, PathBuf>,
) -> Result<()> {
    for _ in 0..8 {
        let mut changed = false;
        for file in files.iter().filter(|file| is_css_asset(&file.relative)) {
            let original_key = normalize_asset_path(&file.relative);
            let output_relative = output_by_original
                .get(&original_key)
                .cloned()
                .unwrap_or_else(|| file.relative.clone());
            let rewritten = rewrite_css_asset_urls(file, &output_relative, output_by_original)?;
            let next_output = fingerprinted_path(&file.relative, &rewritten);
            if output_by_original.get(&original_key) != Some(&next_output) {
                output_by_original.insert(original_key, next_output);
                changed = true;
            }
        }
        if !changed {
            return Ok(());
        }
    }
    bail!("theme CSS asset fingerprints did not stabilize");
}

fn remove_stale_theme_assets(dest: &Path, current_outputs: &HashSet<String>) -> Result<()> {
    if !dest.exists() {
        return Ok(());
    }
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(dest) {
        let entry = entry.with_context(|| format!("failed to walk {}", dest.display()))?;
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    for path in files {
        let relative = path.strip_prefix(dest)?;
        if current_outputs.contains(&normalize_asset_path(relative)) {
            continue;
        }
        fs::remove_file(&path).with_context(|| format!("failed to remove {}", path.display()))?;
        remove_empty_parents(dest, path.parent())?;
    }
    Ok(())
}

fn fingerprinted_path(relative: &Path, bytes: &[u8]) -> PathBuf {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let hash = format!("{:x}", hasher.finalize());
    let short = &hash[..12];
    let stem = relative
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("asset");
    let ext = relative.extension().and_then(|value| value.to_str());
    let file_name = match ext {
        Some(ext) => format!("{stem}.{short}.{ext}"),
        None => format!("{stem}.{short}"),
    };
    relative
        .parent()
        .map(|parent| parent.join(&file_name))
        .unwrap_or_else(|| PathBuf::from(file_name))
}

fn remove_empty_parents(root: &Path, mut parent: Option<&Path>) -> Result<()> {
    while let Some(path) = parent {
        if path == root {
            break;
        }
        if fs::read_dir(path)
            .with_context(|| format!("failed to read {}", path.display()))?
            .next()
            .is_none()
        {
            fs::remove_dir(path).with_context(|| format!("failed to remove {}", path.display()))?;
            parent = path.parent();
        } else {
            break;
        }
    }
    Ok(())
}

fn is_css_asset(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("css"))
}

fn rewrite_css_asset_urls(
    file: &ThemeAssetFile,
    output_relative: &Path,
    output_by_original: &BTreeMap<String, PathBuf>,
) -> Result<Vec<u8>> {
    let css = std::str::from_utf8(&file.bytes).with_context(|| {
        format!(
            "theme CSS is not valid UTF-8: {}",
            file.source_path.display()
        )
    })?;
    Ok(rewrite_css_urls(css, output_relative, output_by_original).into_bytes())
}

fn rewrite_css_urls(
    css: &str,
    css_output_relative: &Path,
    output_by_original: &BTreeMap<String, PathBuf>,
) -> String {
    let mut output = String::with_capacity(css.len());
    let mut index = 0;
    while let Some(relative_start) = find_ascii_case_insensitive(&css[index..], "url(") {
        let start = index + relative_start;
        let value_start = start + "url(".len();
        let Some(relative_end) = css[value_start..].find(')') else {
            break;
        };
        let value_end = value_start + relative_end;
        output.push_str(&css[index..value_start]);
        let value = &css[value_start..value_end];
        if let Some(rewritten) =
            rewrite_css_url_value(value, css_output_relative, output_by_original)
        {
            output.push_str(&rewritten);
        } else {
            output.push_str(value);
        }
        output.push(')');
        index = value_end + 1;
    }
    output.push_str(&css[index..]);
    output
}

fn rewrite_css_url_value(
    value: &str,
    css_output_relative: &Path,
    output_by_original: &BTreeMap<String, PathBuf>,
) -> Option<String> {
    let leading_len = value.len() - value.trim_start().len();
    let trailing_len = value.len() - value.trim_end().len();
    let leading = &value[..leading_len];
    let trailing = &value[value.len().saturating_sub(trailing_len)..];
    let trimmed = value.trim();
    let (quote, inner) = if trimmed.len() >= 2
        && ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
    {
        (&trimmed[..1], &trimmed[1..trimmed.len() - 1])
    } else {
        ("", trimmed)
    };
    let (path_part, suffix) = split_css_url_suffix(inner);
    if path_part.is_empty()
        || path_part.starts_with('/')
        || path_part.starts_with('#')
        || has_uri_scheme(path_part)
    {
        return None;
    }

    let css_parent = css_output_relative
        .parent()
        .unwrap_or_else(|| Path::new(""));
    let target_original = normalize_relative_asset_path(&css_parent.join(path_part));
    let target_output = output_by_original.get(&normalize_asset_path(&target_original))?;
    let rewritten_path = relative_asset_url(css_parent, target_output);
    Some(format!(
        "{leading}{quote}{rewritten_path}{suffix}{quote}{trailing}"
    ))
}

fn split_css_url_suffix(raw_url: &str) -> (&str, &str) {
    let hash = raw_url.find('#');
    let query = raw_url.find('?');
    let split_at = match (hash, query) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) | (None, Some(a)) => a,
        (None, None) => raw_url.len(),
    };
    raw_url.split_at(split_at)
}

fn relative_asset_url(from_dir: &Path, target: &Path) -> String {
    let from = normal_components(from_dir);
    let to = normal_components(target);
    let common = from
        .iter()
        .zip(&to)
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts = Vec::new();
    parts.extend(std::iter::repeat_n("..".to_string(), from.len() - common));
    parts.extend(to[common..].iter().cloned());
    if parts.is_empty() {
        ".".to_string()
    } else {
        parts.join("/")
    }
}

fn normalize_relative_asset_path(path: &Path) -> PathBuf {
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

fn normal_components(path: &Path) -> Vec<String> {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect()
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle.as_bytes()))
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

fn normalize_asset_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}
