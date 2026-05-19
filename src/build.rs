use std::{
    collections::{BTreeMap, HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result, bail};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    config::Config,
    diagnostics::{Diagnostic, Diagnostics},
    markdown::{
        FrontMatter, MarkdownRenderer, collect_page_urls, format_front_matter_error,
        split_front_matter,
    },
    paths::{ensure_relative, is_markdown, is_private_path, normalize_url_path, page_output_path},
    theme::{
        AssetMap, PageContext, PartialUsage, RenderPageInput, Theme, build_context,
        copy_dir_contents, copy_theme_assets_with_options, site_context,
    },
};

#[derive(Debug, Clone, Copy)]
pub struct BuildOptions {
    pub clean: bool,
    pub no_cache: bool,
    pub jobs: Option<usize>,
    pub profile: bool,
}

#[derive(Debug, Default)]
struct BuildStats {
    pages_rendered: usize,
    pages_cached: usize,
    assets_copied: usize,
    assets_cached: usize,
    theme_assets_copied: usize,
    partials_rendered: usize,
    warnings: usize,
    elapsed_ms: u128,
    profile: BuildProfile,
}

#[derive(Debug, Default, Clone, Copy)]
struct DiagnosticStats {
    warnings: usize,
}

#[derive(Debug, Default, Clone, Copy)]
struct BuildProfile {
    setup_ms: u128,
    scan_ms: u128,
    template_load_ms: u128,
    partial_ms: u128,
    fingerprint_ms: u128,
    theme_asset_ms: u128,
    page_render_ms: u128,
    markdown_render_ms: u128,
    katex_render_ms: u128,
    code_highlight_ms: u128,
    layout_render_ms: u128,
    page_write_ms: u128,
    content_asset_ms: u128,
    site_infra_ms: u128,
    cleanup_ms: u128,
    cache_write_ms: u128,
}

#[derive(Debug, Clone)]
struct SourcePage {
    source_path: PathBuf,
    relative_path: PathBuf,
    output_path: PathBuf,
    url: String,
    input_hash: String,
    front_matter: FrontMatter,
}

#[derive(Debug, Clone)]
struct SourceAsset {
    source_path: PathBuf,
    relative_path: PathBuf,
    input_hash: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct CacheManifest {
    renderer_version: String,
    render_fingerprint: String,
    pages: HashMap<String, CacheEntry>,
    assets: HashMap<String, CacheEntry>,
    outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    input_hash: String,
    output_path: String,
    #[serde(default)]
    diagnostics: Vec<Diagnostic>,
    #[serde(default)]
    partials_used: Vec<String>,
}

pub fn build_site(config: &Config, options: BuildOptions) -> Result<()> {
    if let Some(jobs) = options.jobs {
        let _ = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .build_global();
    }

    let start = Instant::now();
    validate_paths(config)?;

    if options.clean {
        clean_site(config)?;
    }
    fs::create_dir_all(&config.paths.output)
        .with_context(|| format!("failed to create {}", config.paths.output.display()))?;
    fs::create_dir_all(&config.paths.cache)
        .with_context(|| format!("failed to create {}", config.paths.cache.display()))?;
    let setup_ms = start.elapsed().as_millis();

    let template_load_start = Instant::now();
    let theme = Theme::load(&config.paths.theme)?;
    let template_load_ms = template_load_start.elapsed().as_millis();

    let scan_start = Instant::now();
    let discovered = discover_sources(config)?;
    detect_output_collisions(config, &discovered)?;
    let scan_ms = scan_start.elapsed().as_millis();

    let page_paths: Vec<PathBuf> = discovered
        .pages
        .iter()
        .map(|page| page.relative_path.clone())
        .collect();
    let page_urls = collect_page_urls(&page_paths, config.theme.url_mode)?;
    let template_page_urls = page_urls
        .iter()
        .map(|(path, url)| (normalize_url_path(path), url.clone()))
        .collect::<HashMap<_, _>>();
    let renderer = MarkdownRenderer::new(config, page_urls);

    let partial_start = Instant::now();
    let (partials, partial_diagnostics) = render_partials(config, &renderer)?;
    let partial_usage = PartialUsage::default();
    let partial_ms = partial_start.elapsed().as_millis();

    let fingerprint_start = Instant::now();
    let render_fingerprint = render_fingerprint(config, &partials)?;
    let old_cache = if options.no_cache {
        CacheManifest::default()
    } else {
        load_cache(config).unwrap_or_default()
    };
    let fingerprint_ms = fingerprint_start.elapsed().as_millis();

    let mut page_jobs = Vec::new();
    let mut cached_page_diagnostics = Diagnostics::default();
    let mut pages_cached = 0;
    for page in &discovered.pages {
        if let Some(entry) = page_cache_entry(
            config,
            &old_cache,
            page,
            &render_fingerprint,
            options.no_cache,
        ) {
            pages_cached += 1;
            cached_page_diagnostics
                .items
                .extend(entry.diagnostics.iter().cloned());
            for key in &entry.partials_used {
                partial_usage.mark(key);
            }
        } else {
            page_jobs.push(page.clone());
        }
    }
    let cached_page_warnings = cached_page_diagnostics.warning_count();
    cached_page_diagnostics.print();

    let theme_asset_start = Instant::now();
    let theme_asset_copy =
        copy_theme_assets_with_options(&theme, &config.paths.output, config.assets.fingerprint)?;
    let theme_assets = theme_asset_copy.assets.clone();
    let theme_asset_ms = theme_asset_start.elapsed().as_millis();

    let page_render_start = Instant::now();
    let rendered_pages: Vec<_> = page_jobs
        .par_iter()
        .map(|page| {
            let page_partial_usage = PartialUsage::default();
            render_page(
                config,
                RenderPageDeps {
                    theme: &theme,
                    renderer: &renderer,
                    partials: &partials,
                    assets: &theme_assets,
                    page_urls: &template_page_urls,
                },
                page,
                page_partial_usage,
            )
        })
        .collect::<Result<Vec<_>>>()?;
    for rendered in &rendered_pages {
        for key in &rendered.partials_used {
            partial_usage.mark(key);
        }
    }
    let unused_partial_diagnostics = warn_unused_partials(&partials, &partial_usage);
    let render_metrics = renderer.metrics_snapshot();
    let page_warnings: usize = rendered_pages
        .iter()
        .map(|rendered| rendered.diagnostics.warnings)
        .sum();
    let layout_render_ms: u128 = rendered_pages
        .iter()
        .map(|rendered| rendered.layout_ms)
        .sum();
    let page_render_ms = page_render_start.elapsed().as_millis();

    let page_write_start = Instant::now();
    let rendered_page_cache_updates = rendered_pages
        .iter()
        .map(|rendered| {
            (
                normalize_url_path(&rendered.relative_path),
                PageCacheUpdate {
                    diagnostics: rendered.diagnostics.items.clone(),
                    partials_used: rendered.partials_used.clone(),
                },
            )
        })
        .collect::<HashMap<_, _>>();
    for rendered in rendered_pages {
        let target = config.paths.output.join(&rendered.output_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&target, rendered.html)
            .with_context(|| format!("failed to write {}", target.display()))?;
    }
    let page_write_ms = page_write_start.elapsed().as_millis();

    let content_asset_start = Instant::now();
    let mut assets_copied = 0;
    let mut assets_cached = 0;
    for asset in &discovered.assets {
        let target = config.paths.output.join(&asset.relative_path);
        if asset_cache_hit(config, &old_cache, asset, options.no_cache) {
            assets_cached += 1;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::copy(&asset.source_path, &target).with_context(|| {
            format!(
                "failed to copy {} to {}",
                asset.source_path.display(),
                target.display()
            )
        })?;
        assets_copied += 1;
    }
    let content_asset_ms = content_asset_start.elapsed().as_millis();

    let theme_assets_copied = theme_asset_copy.count;
    let site_infra_start = Instant::now();
    let infra_count = write_site_infra(config, &discovered.pages)?;
    let site_infra_ms = site_infra_start.elapsed().as_millis();
    let cleanup_start = Instant::now();
    cleanup_orphans(config, &old_cache, &discovered)?;
    let cleanup_ms = cleanup_start.elapsed().as_millis();
    let cache_write_start = Instant::now();
    write_manifest(
        config,
        &discovered,
        &old_cache,
        &rendered_page_cache_updates,
        render_fingerprint,
    )?;
    renderer.write_persistent_cache()?;
    let cache_write_ms = cache_write_start.elapsed().as_millis();

    let stats = BuildStats {
        pages_rendered: page_jobs.len(),
        pages_cached,
        assets_copied,
        assets_cached,
        theme_assets_copied,
        partials_rendered: partials.len(),
        warnings: partial_diagnostics.warnings
            + page_warnings
            + cached_page_warnings
            + unused_partial_diagnostics.warnings,
        elapsed_ms: start.elapsed().as_millis(),
        profile: BuildProfile {
            setup_ms,
            scan_ms,
            template_load_ms,
            partial_ms,
            fingerprint_ms,
            theme_asset_ms,
            page_render_ms,
            markdown_render_ms: render_metrics.markdown_ms as u128,
            katex_render_ms: render_metrics.katex_ms as u128,
            code_highlight_ms: render_metrics.code_highlight_ms as u128,
            layout_render_ms,
            page_write_ms,
            content_asset_ms,
            site_infra_ms,
            cleanup_ms,
            cache_write_ms,
        },
    };
    print_summary(&stats, infra_count, options);
    Ok(())
}

pub fn clean_site(config: &Config) -> Result<()> {
    if config.paths.output.exists() {
        fs::remove_dir_all(&config.paths.output)
            .with_context(|| format!("failed to remove {}", config.paths.output.display()))?;
    }
    Ok(())
}

struct DiscoveredSources {
    pages: Vec<SourcePage>,
    assets: Vec<SourceAsset>,
}

struct RenderedPage {
    relative_path: PathBuf,
    output_path: PathBuf,
    html: String,
    diagnostics: RenderDiagnostics,
    partials_used: Vec<String>,
    layout_ms: u128,
}

struct RenderDiagnostics {
    warnings: usize,
    items: Vec<Diagnostic>,
}

struct RenderPageDeps<'a> {
    theme: &'a Theme,
    renderer: &'a MarkdownRenderer,
    partials: &'a BTreeMap<String, String>,
    assets: &'a AssetMap,
    page_urls: &'a HashMap<String, String>,
}

struct PageCacheUpdate {
    diagnostics: Vec<Diagnostic>,
    partials_used: Vec<String>,
}

fn validate_paths(config: &Config) -> Result<()> {
    ensure_relative(&config.paths.content)?;
    ensure_relative(&config.paths.output)?;
    ensure_relative(&config.paths.theme)?;
    if !config.paths.content.exists() {
        bail!(
            "content directory does not exist: {}",
            config.paths.content.display()
        );
    }
    if !config.paths.theme.exists() {
        bail!(
            "theme directory does not exist: {}",
            config.paths.theme.display()
        );
    }
    Ok(())
}

fn discover_sources(config: &Config) -> Result<DiscoveredSources> {
    let mut pages = Vec::new();
    let mut assets = Vec::new();

    for entry in walkdir::WalkDir::new(&config.paths.content) {
        let entry =
            entry.with_context(|| format!("failed to walk {}", config.paths.content.display()))?;
        if entry.file_type().is_dir() {
            continue;
        }
        let source_path = entry.path().to_path_buf();
        let relative_path = source_path
            .strip_prefix(&config.paths.content)
            .with_context(|| format!("failed to relativize {}", source_path.display()))?
            .to_path_buf();

        if is_private_path(&relative_path) {
            continue;
        }

        if is_markdown(&relative_path) {
            reject_legacy_protect_page(&relative_path)?;
            let raw = fs::read_to_string(&source_path)
                .with_context(|| format!("failed to read {}", source_path.display()))?;
            let (front_matter, _, _) = split_front_matter(&raw).map_err(|err| {
                format_front_matter_error(&source_path, &relative_path, &raw, err)
            })?;
            if front_matter.draft.unwrap_or(false) {
                continue;
            }
            let output_path = page_output_path(&relative_path, config.theme.url_mode)?;
            let url = crate::paths::page_public_url(&relative_path, config.theme.url_mode)?;
            let input_hash = hash_bytes(raw.as_bytes());
            pages.push(SourcePage {
                source_path,
                relative_path,
                output_path,
                url,
                input_hash,
                front_matter,
            });
        } else {
            let input_hash = hash_file(&source_path)?;
            assets.push(SourceAsset {
                source_path,
                relative_path,
                input_hash,
            });
        }
    }

    Ok(DiscoveredSources { pages, assets })
}

fn reject_legacy_protect_page(relative_path: &Path) -> Result<()> {
    let Some(file_name) = relative_path.file_name().and_then(|name| name.to_str()) else {
        return Ok(());
    };
    if file_name.ends_with(".protect.md") || file_name.ends_with(".protect.markdown") {
        bail!(
            "legacy protected page is not supported in the Rust rewrite: {}. Move it under a private `_` directory or rename it if it should be published as a normal page.",
            relative_path.display()
        );
    }
    Ok(())
}

fn detect_output_collisions(config: &Config, discovered: &DiscoveredSources) -> Result<()> {
    let mut seen = HashMap::<String, String>::new();
    for page in &discovered.pages {
        remember_output(
            &mut seen,
            &page.output_path,
            format!("page {}", page.relative_path.display()),
        )?;
    }
    for asset in &discovered.assets {
        if config.paths.theme.join("assets").exists()
            && normalize_url_path(&asset.relative_path).starts_with("assets/theme/")
        {
            bail!(
                "duplicate output path: {} is reserved for theme assets",
                asset.relative_path.display()
            );
        }
        remember_output(
            &mut seen,
            &asset.relative_path,
            format!("asset {}", asset.relative_path.display()),
        )?;
    }
    if config.site_infra.robots {
        remember_output(
            &mut seen,
            Path::new("robots.txt"),
            "generated robots.txt".to_string(),
        )?;
    }
    if config.site_infra.sitemap {
        remember_output(
            &mut seen,
            Path::new("sitemap.xml"),
            "generated sitemap.xml".to_string(),
        )?;
    }
    Ok(())
}

fn remember_output(
    seen: &mut HashMap<String, String>,
    output_path: &Path,
    source: String,
) -> Result<()> {
    let output = normalize_url_path(output_path);
    if let Some(previous) = seen.insert(output.clone(), source.clone()) {
        bail!("duplicate output path: {output} from {previous} and {source}");
    }
    Ok(())
}

fn render_partials(
    config: &Config,
    renderer: &MarkdownRenderer,
) -> Result<(BTreeMap<String, String>, DiagnosticStats)> {
    let partial_root = config.paths.content.join("_partials");
    let mut partials = BTreeMap::new();
    let mut partial_sources = BTreeMap::<String, PathBuf>::new();
    let mut stats = DiagnosticStats::default();
    if !partial_root.exists() {
        return Ok((partials, stats));
    }

    for entry in walkdir::WalkDir::new(&partial_root) {
        let entry = entry.with_context(|| format!("failed to walk {}", partial_root.display()))?;
        if entry.file_type().is_dir() {
            continue;
        }
        let path = entry.path();
        let relative = path.strip_prefix(&partial_root)?;
        let key = partial_key(relative);
        if let Some(previous) = partial_sources.get(&key) {
            bail!(
                "duplicate partial key `{}` from {} and {}. Rename one partial so each key is unique.",
                key,
                previous.display(),
                relative.display()
            );
        }
        partial_sources.insert(key.clone(), relative.to_path_buf());
        if is_markdown(path) {
            let raw = fs::read_to_string(path)
                .with_context(|| format!("failed to read partial {}", path.display()))?;
            let mut diagnostics = Diagnostics::default();
            let rendered = renderer.render(path, relative, &raw, &mut diagnostics)?;
            stats.warnings += diagnostics.warning_count();
            diagnostics.print();
            partials.insert(key, rendered.html);
        } else if path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("html"))
        {
            partials.insert(
                key,
                fs::read_to_string(path)
                    .with_context(|| format!("failed to read partial {}", path.display()))?,
            );
        }
    }

    Ok((partials, stats))
}

fn render_page(
    config: &Config,
    deps: RenderPageDeps<'_>,
    page: &SourcePage,
    partial_usage: PartialUsage,
) -> Result<RenderedPage> {
    let raw = fs::read_to_string(&page.source_path)
        .with_context(|| format!("failed to read {}", page.source_path.display()))?;
    let mut diagnostics = Diagnostics::default();
    let rendered = deps.renderer.render(
        &page.source_path,
        &page.relative_path,
        &raw,
        &mut diagnostics,
    )?;
    let diagnostic_stats = DiagnosticStats {
        warnings: diagnostics.warning_count(),
    };
    let diagnostic_items = diagnostics.items.clone();
    diagnostics.print();

    let title = rendered
        .front_matter
        .title
        .clone()
        .or_else(|| page.front_matter.title.clone());
    let layout = rendered
        .front_matter
        .layout
        .clone()
        .or_else(|| page.front_matter.layout.clone())
        .unwrap_or_else(|| config.theme.default_layout.clone());

    let rendered_template = deps.theme.render_page(RenderPageInput {
        layout: layout.clone(),
        site: site_context(config),
        page: PageContext {
            title,
            source_path: normalize_url_path(&page.relative_path),
            output_path: normalize_url_path(&page.output_path),
            url: page.url.clone(),
            layout,
        },
        content: rendered.html,
        partials: deps.partials.clone(),
        partial_usage: partial_usage.clone(),
        assets: deps.assets.clone(),
        page_urls: deps.page_urls.clone(),
        renderer: deps.renderer.clone(),
        build: build_context(),
    })?;

    Ok(RenderedPage {
        relative_path: page.relative_path.clone(),
        output_path: page.output_path.clone(),
        html: rendered_template.html,
        diagnostics: RenderDiagnostics {
            warnings: diagnostic_stats.warnings,
            items: diagnostic_items,
        },
        partials_used: sorted_strings(partial_usage.used_keys()),
        layout_ms: rendered_template.layout_ms,
    })
}

fn warn_unused_partials(
    partials: &BTreeMap<String, String>,
    partial_usage: &PartialUsage,
) -> DiagnosticStats {
    let used = partial_usage.used_keys();
    let mut diagnostics = Diagnostics::default();
    for key in partials.keys() {
        if used.contains(key) {
            continue;
        }
        diagnostics.warn(
            Some(PathBuf::from("_partials")),
            format!("unused partial: {key}"),
        );
    }
    let stats = DiagnosticStats {
        warnings: diagnostics.warning_count(),
    };
    diagnostics.print();
    stats
}

fn partial_key(relative: &Path) -> String {
    let mut path = relative.with_extension("");
    if path.file_name().is_some_and(|name| name == "index") {
        path.pop();
    }
    normalize_url_path(&path).replace('/', ".")
}

fn write_site_infra(config: &Config, pages: &[SourcePage]) -> Result<usize> {
    let mut count = 0;
    if config.site_infra.robots {
        let mut robots = String::from("User-agent: *\nAllow: /\n");
        if config.site_infra.sitemap {
            robots.push_str("Sitemap: /sitemap.xml\n");
        }
        fs::write(config.paths.output.join("robots.txt"), robots)
            .context("failed to write robots.txt")?;
        count += 1;
    }
    if config.site_infra.sitemap {
        let mut body = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        body.push_str("\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
        for page in pages {
            let loc = join_base_url(&config.site.base_url, &page.url);
            body.push_str("  <url><loc>");
            body.push_str(&html_escape::encode_text(&loc));
            body.push_str("</loc></url>\n");
        }
        body.push_str("</urlset>\n");
        fs::write(config.paths.output.join("sitemap.xml"), body)
            .context("failed to write sitemap.xml")?;
        count += 1;
    }
    Ok(count)
}

fn write_manifest(
    config: &Config,
    discovered: &DiscoveredSources,
    old_cache: &CacheManifest,
    rendered_page_updates: &HashMap<String, PageCacheUpdate>,
    render_fingerprint: String,
) -> Result<()> {
    let mut pages = HashMap::new();
    let mut assets = HashMap::new();
    let mut outputs = Vec::new();

    for page in &discovered.pages {
        let key = normalize_url_path(&page.relative_path);
        let output_path = normalize_url_path(&page.output_path);
        outputs.push(output_path.clone());
        let diagnostics = rendered_page_updates
            .get(&key)
            .map(|entry| entry.diagnostics.clone())
            .or_else(|| {
                old_cache
                    .pages
                    .get(&key)
                    .map(|entry| entry.diagnostics.clone())
            })
            .unwrap_or_default();
        let partials_used = rendered_page_updates
            .get(&key)
            .map(|entry| entry.partials_used.clone())
            .or_else(|| {
                old_cache
                    .pages
                    .get(&key)
                    .map(|entry| entry.partials_used.clone())
            })
            .unwrap_or_default();
        pages.insert(
            key.clone(),
            CacheEntry {
                input_hash: page.input_hash.clone(),
                output_path,
                diagnostics,
                partials_used,
            },
        );
    }

    for asset in &discovered.assets {
        let key = normalize_url_path(&asset.relative_path);
        outputs.push(key.clone());
        assets.insert(
            key.clone(),
            CacheEntry {
                input_hash: asset.input_hash.clone(),
                output_path: key,
                diagnostics: Vec::new(),
                partials_used: Vec::new(),
            },
        );
    }

    if config.site_infra.robots {
        outputs.push("robots.txt".to_string());
    }
    if config.site_infra.sitemap {
        outputs.push("sitemap.xml".to_string());
    }

    let manifest = CacheManifest {
        renderer_version: env!("CARGO_PKG_VERSION").to_string(),
        render_fingerprint,
        pages,
        assets,
        outputs,
    };

    let path = config.paths.cache.join("manifest.json");
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    fs::write(&path, serde_json::to_vec_pretty(&manifest)?)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn load_cache(config: &Config) -> Result<CacheManifest> {
    let path = config.paths.cache.join("manifest.json");
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

fn page_cache_entry<'a>(
    config: &Config,
    cache: &'a CacheManifest,
    page: &SourcePage,
    render_fingerprint: &str,
    no_cache: bool,
) -> Option<&'a CacheEntry> {
    if no_cache
        || cache.renderer_version != env!("CARGO_PKG_VERSION")
        || cache.render_fingerprint != render_fingerprint
    {
        return None;
    }
    let key = normalize_url_path(&page.relative_path);
    let output_path = normalize_url_path(&page.output_path);
    let entry = cache.pages.get(&key)?;
    (entry.input_hash == page.input_hash
        && entry.output_path == output_path
        && config.paths.output.join(&page.output_path).exists())
    .then_some(entry)
}

fn asset_cache_hit(
    config: &Config,
    cache: &CacheManifest,
    asset: &SourceAsset,
    no_cache: bool,
) -> bool {
    if no_cache {
        return false;
    }
    let key = normalize_url_path(&asset.relative_path);
    let Some(entry) = cache.assets.get(&key) else {
        return false;
    };
    entry.input_hash == asset.input_hash && config.paths.output.join(&asset.relative_path).exists()
}

fn cleanup_orphans(
    config: &Config,
    old_cache: &CacheManifest,
    discovered: &DiscoveredSources,
) -> Result<()> {
    let mut current_outputs = HashSet::new();
    for page in &discovered.pages {
        current_outputs.insert(normalize_url_path(&page.output_path));
    }
    for asset in &discovered.assets {
        current_outputs.insert(normalize_url_path(&asset.relative_path));
    }
    if config.site_infra.robots {
        current_outputs.insert("robots.txt".to_string());
    }
    if config.site_infra.sitemap {
        current_outputs.insert("sitemap.xml".to_string());
    }

    for output in &old_cache.outputs {
        if current_outputs.contains(output) {
            continue;
        }
        let path = config.paths.output.join(output);
        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove orphan output {}", path.display()))?;
            remove_empty_parents(&config.paths.output, path.parent())?;
        }
    }
    Ok(())
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

fn render_fingerprint(config: &Config, partials: &BTreeMap<String, String>) -> Result<String> {
    let mut hasher = Sha256::new();
    hasher.update(env!("CARGO_PKG_VERSION").as_bytes());
    hasher.update(serde_json::to_vec(config)?);
    for (key, value) in partials {
        hasher.update(key.as_bytes());
        hasher.update(value.as_bytes());
    }
    hash_dir_into(&config.paths.theme, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn hash_dir_into(path: &Path, hasher: &mut Sha256) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry.with_context(|| format!("failed to walk {}", path.display()))?;
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    for file in files {
        let relative = file.strip_prefix(path)?;
        hasher.update(normalize_url_path(relative).as_bytes());
        hasher
            .update(fs::read(&file).with_context(|| format!("failed to read {}", file.display()))?);
    }
    Ok(())
}

fn hash_file(path: &Path) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(hash_bytes(&bytes))
}

fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn sorted_strings(values: HashSet<String>) -> Vec<String> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort();
    values
}

fn join_base_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = path.trim_start_matches('/');
    if base.is_empty() {
        format!("/{path}")
    } else {
        format!("{base}/{path}")
    }
}

fn print_summary(stats: &BuildStats, infra_count: usize, options: BuildOptions) {
    println!(
        "built {} pages, copied {} assets and {} theme assets, rendered {} partials, generated {} site files in {}ms",
        stats.pages_rendered,
        stats.assets_copied,
        stats.theme_assets_copied,
        stats.partials_rendered,
        infra_count,
        stats.elapsed_ms
    );
    if options.profile {
        println!("profile:");
        println!("  total: {}ms", stats.elapsed_ms);
        println!("  pages rendered: {}", stats.pages_rendered);
        println!("  pages cached: {}", stats.pages_cached);
        println!("  partials rendered: {}", stats.partials_rendered);
        println!("  assets copied: {}", stats.assets_copied);
        println!("  assets cached: {}", stats.assets_cached);
        println!("  theme assets copied: {}", stats.theme_assets_copied);
        println!("  setup: {}ms", stats.profile.setup_ms);
        println!("  source scan: {}ms", stats.profile.scan_ms);
        println!("  template load: {}ms", stats.profile.template_load_ms);
        println!("  partial render: {}ms", stats.profile.partial_ms);
        println!(
            "  fingerprint/cache load: {}ms",
            stats.profile.fingerprint_ms
        );
        println!("  theme asset copy: {}ms", stats.profile.theme_asset_ms);
        println!("  page render: {}ms", stats.profile.page_render_ms);
        println!("  markdown render: {}ms", stats.profile.markdown_render_ms);
        println!("  katex render: {}ms", stats.profile.katex_render_ms);
        println!("  code highlighting: {}ms", stats.profile.code_highlight_ms);
        println!("  layout render: {}ms", stats.profile.layout_render_ms);
        println!("  page write: {}ms", stats.profile.page_write_ms);
        println!("  content asset copy: {}ms", stats.profile.content_asset_ms);
        println!("  site infrastructure: {}ms", stats.profile.site_infra_ms);
        println!("  orphan cleanup: {}ms", stats.profile.cleanup_ms);
        println!("  cache write: {}ms", stats.profile.cache_write_ms);
        println!("  warnings: {}", stats.warnings);
        if options.no_cache {
            println!("  cache mode: disabled by --no-cache");
        }
    }
}

#[allow(dead_code)]
fn _copy_content_assets(source: &Path, dest: &Path) -> Result<usize> {
    copy_dir_contents(source, dest)
}
