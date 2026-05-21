use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn build(project: &std::path::Path, args: &[&str]) -> assert_cmd::assert::Assert {
    let mut command = Command::cargo_bin("blogx").unwrap();
    command.current_dir(project).arg("build").args(args);
    command.assert()
}

#[test]
fn init_and_build_site() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success()
        .stdout(predicate::str::contains("initialized"));

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .arg("build")
        .assert()
        .success()
        .stdout(predicate::str::contains("built 3 pages"));

    assert!(project.join("public/index.html").exists());
    assert!(project.join("public/about.html").exists());
    assert!(project.join("public/notes/hello.html").exists());
    assert!(project.join("content/_drafts/unfinished.md").exists());
    assert!(project.join("content/_partials/sidebar.md").exists());
    assert!(project.join("public/assets/theme/css/main.css").exists());
    assert!(project.join("public/sitemap.xml").exists());
    assert!(project.join("public/robots.txt").exists());
    assert!(project.join("theme/partials/head.html").exists());
    assert!(project.join("theme/partials/header.html").exists());
    assert!(project.join("theme/partials/footer.html").exists());
    assert!(project.join("theme/partials/nav.html").exists());
    assert!(
        project
            .join("public/assets/theme/js/copy-tex.min.js")
            .exists()
    );
    assert!(project.join("public/assets/theme/js/enhance.js").exists());

    let sitemap = std::fs::read_to_string(project.join("public/sitemap.xml")).unwrap();
    assert!(sitemap.contains("<loc>/index.html</loc>"));
    assert!(sitemap.contains("<loc>/about.html</loc>"));

    let robots = std::fs::read_to_string(project.join("public/robots.txt")).unwrap();
    assert!(robots.contains("User-agent: *"));
    assert!(robots.contains("Sitemap: /sitemap.xml"));

    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.contains(r#"<meta name="generator" content="BlogX"#));
    assert!(html.contains(r#"<script src="/assets/theme/js/copy-tex.min.js" defer></script>"#));
    assert!(html.contains(r#"<nav class="site-nav" aria-label="Primary">"#));
    assert!(html.contains(r#"<aside class="sidebar" aria-label="Sidebar">"#));

    let enhance =
        std::fs::read_to_string(project.join("public/assets/theme/js/enhance.js")).unwrap();
    let main_css =
        std::fs::read_to_string(project.join("public/assets/theme/css/main.css")).unwrap();
    assert!(enhance.contains("enableWheelHorizontalScroll(block);"));
    assert!(enhance.contains("frame.append(button);"));
    assert!(enhance.contains("enableWheelHorizontalScroll(katexDisplay);"));
    assert!(main_css.contains(".code-block"));
    assert!(main_css.contains(".code-block pre"));
    assert!(main_css.contains("position: sticky"));
    assert!(main_css.contains("padding-left: 1rem"));
    assert!(main_css.contains("left: 0"));
}

#[test]
fn theme_sync_updates_configured_theme_directory() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    let config = project.join("blogx.toml");
    let raw = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        raw.replace("theme = \"theme\"", "theme = \"custom-theme\""),
    )
    .unwrap();
    std::fs::rename(project.join("theme"), project.join("custom-theme")).unwrap();

    let main_css = project.join("custom-theme/assets/css/main.css");
    std::fs::write(&main_css, "stale css").unwrap();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .args(["theme", "sync"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "synced default theme to custom-theme",
        ));

    let restored = std::fs::read_to_string(main_css).unwrap();
    assert!(restored.contains(r#"font-family: "LXGW WenKai TC";"#));
}

#[test]
fn theme_sync_prune_removes_extra_theme_files() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    let stale = project.join("theme/assets/css/old.css");
    std::fs::write(&stale, "old").unwrap();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .args(["theme", "sync", "--prune"])
        .assert()
        .success()
        .stdout(predicate::str::contains("files pruned"));

    assert!(!stale.exists());
    assert!(project.join("theme/assets/css/main.css").exists());
}

#[test]
fn robots_omits_sitemap_when_sitemap_generation_is_disabled() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    let config = project.join("blogx.toml");
    let raw = std::fs::read_to_string(&config).unwrap();
    std::fs::write(&config, raw.replace("sitemap = true", "sitemap = false")).unwrap();

    build(&project, &[]).success();

    assert!(!project.join("public/sitemap.xml").exists());
    let robots = std::fs::read_to_string(project.join("public/robots.txt")).unwrap();
    assert!(robots.contains("User-agent: *"));
    assert!(!robots.contains("Sitemap: /sitemap.xml"));
}

#[test]
fn build_does_not_generate_deferred_visible_or_feed_outputs() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    build(&project, &[]).success();

    for path in [
        "public/rss.xml",
        "public/feed.xml",
        "public/feed.json",
        "public/search.json",
        "public/search/index.html",
        "public/tags/index.html",
        "public/categories/index.html",
        "public/archive/index.html",
    ] {
        assert!(
            !project.join(path).exists(),
            "{path} should not be generated"
        );
    }
}

#[test]
fn clean_url_mode_outputs_index_pages() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    let config = project.join("blogx.toml");
    let raw = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        raw.replace("url_mode = \"html\"", "url_mode = \"clean\""),
    )
    .unwrap();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .arg("build")
        .assert()
        .success();

    assert!(project.join("public/about/index.html").exists());
    let index = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(index.contains("about/"));
}

#[test]
fn cached_rebuild_skips_unchanged_pages_and_cleans_deleted_outputs() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .args(["build", "--profile"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pages rendered: 3"));

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .args(["build", "--profile"])
        .assert()
        .success()
        .stdout(predicate::str::contains("pages rendered: 0"))
        .stdout(predicate::str::contains("pages cached: 3"));

    std::fs::remove_file(project.join("content/about.md")).unwrap();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .args(["build", "--profile"])
        .assert()
        .success();

    assert!(!project.join("public/about.html").exists());
}

#[test]
fn duplicate_page_asset_and_site_infra_outputs_are_errors() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(project.join("content/about.html"), "<p>asset</p>").unwrap();
    build(&project, &[])
        .failure()
        .stderr(predicate::str::contains(
            "duplicate output path: about.html",
        ))
        .stderr(predicate::str::contains("page about.md"))
        .stderr(predicate::str::contains("asset about.html"));

    std::fs::remove_file(project.join("content/about.html")).unwrap();
    std::fs::write(project.join("content/sitemap.xml"), "<xml />").unwrap();
    build(&project, &[])
        .failure()
        .stderr(predicate::str::contains(
            "duplicate output path: sitemap.xml",
        ))
        .stderr(predicate::str::contains("asset sitemap.xml"))
        .stderr(predicate::str::contains("generated sitemap.xml"));

    std::fs::remove_file(project.join("content/sitemap.xml")).unwrap();
    std::fs::create_dir_all(project.join("content/assets/theme/css")).unwrap();
    std::fs::write(project.join("content/assets/theme/css/main.css"), "body{}").unwrap();
    build(&project, &[])
        .failure()
        .stderr(predicate::str::contains(
            "assets/theme/css/main.css is reserved for theme assets",
        ));
}

#[test]
fn editing_one_page_rebuilds_only_that_page() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    build(&project, &["--profile"]).success();
    std::fs::write(
        project.join("content/about.md"),
        "---\ntitle: About\n---\n# About\n\nUpdated page.\n",
    )
    .unwrap();

    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 1"))
        .stdout(predicate::str::contains("pages cached: 2"));
}

#[test]
fn build_jobs_flag_and_clean_command_work() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .args(["build", "--jobs", "2"])
        .assert()
        .success();
    assert!(project.join("public/index.html").exists());

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .arg("clean")
        .assert()
        .success();
    assert!(!project.join("public").exists());
}

#[test]
fn broken_markdown_links_warn_but_build() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(project.join("content/index.md"), "[Missing](missing.md)\n").unwrap();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .args(["build", "--profile", "--no-cache"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "warning: index.md:1:11: local markdown link target does not exist: missing.md",
        ))
        .stderr(predicate::str::contains("  1 | [Missing](missing.md)"))
        .stderr(predicate::str::contains(
            "help: create the target Markdown file or update the link to an existing page",
        ))
        .stdout(predicate::str::contains("warnings: 1"));
}

#[test]
fn cached_pages_replay_markdown_diagnostics() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("content/index.md"),
        "---\ntitle: Home\nkind: post\n---\n# Home\n\n[Missing](missing.md)\n\n![](missing-alt.png)\n",
    )
    .unwrap();

    build(&project, &["--profile"])
        .success()
        .stderr(predicate::str::contains(
            "local markdown link target does not exist: missing.md",
        ))
        .stderr(predicate::str::contains("unknown front matter field: kind"))
        .stderr(predicate::str::contains("image is missing alt text"))
        .stdout(predicate::str::contains("pages rendered: 3"))
        .stdout(predicate::str::contains("warnings: 3"));

    build(&project, &["--profile"])
        .success()
        .stderr(predicate::str::contains(
            "local markdown link target does not exist: missing.md",
        ))
        .stderr(predicate::str::contains("unknown front matter field: kind"))
        .stderr(predicate::str::contains("image is missing alt text"))
        .stdout(predicate::str::contains("pages rendered: 0"))
        .stdout(predicate::str::contains("pages cached: 3"))
        .stdout(predicate::str::contains("warnings: 3"));
}

#[test]
fn uri_scheme_markdown_links_are_not_rewritten_or_warned() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("content/index.md"),
        "[FTP](ftp://example.com/readme.md)\n[Phone](tel:+15551234567)\n[Data](data:text/plain,hello.md)\n[About](about.md)\n",
    )
    .unwrap();

    build(&project, &["--profile", "--no-cache"])
        .success()
        .stderr(predicate::str::is_empty())
        .stdout(predicate::str::contains("warnings: 0"));

    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.contains(r#"<a href="ftp://example.com/readme.md">FTP</a>"#));
    assert!(html.contains(r#"<a href="tel:+15551234567">Phone</a>"#));
    assert!(html.contains(r#"<a href="data:text/plain,hello.md">Data</a>"#));
    assert!(html.contains(r#"<a href="about.html">About</a>"#));
}

#[test]
fn fingerprinted_theme_assets_are_referenced() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    let config = project.join("blogx.toml");
    let raw = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        raw.replace("fingerprint = false", "fingerprint = true"),
    )
    .unwrap();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .arg("build")
        .assert()
        .success();

    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.contains("/assets/theme/css/main."));
    assert!(html.contains(".css"));

    let css_dir = project.join("public/assets/theme/css");
    let has_fingerprinted_css = std::fs::read_dir(css_dir).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".css")
    });
    assert!(has_fingerprinted_css);
}

#[test]
fn fingerprinted_theme_assets_rewrite_css_urls_and_clean_deleted_assets() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    let config = project.join("blogx.toml");
    let raw = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        raw.replace("fingerprint = false", "fingerprint = true"),
    )
    .unwrap();

    build(&project, &[]).success();

    let css_dir = project.join("public/assets/theme/css");
    let katex_css_path = std::fs::read_dir(&css_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("katex.")
        })
        .expect("fingerprinted katex css should exist");
    let katex_css = std::fs::read_to_string(&katex_css_path).unwrap();
    assert!(katex_css.contains("url(fonts/KaTeX_Main-Regular."));
    assert!(!katex_css.contains("url(fonts/KaTeX_Main-Regular.woff2)"));

    let rewritten_font = katex_css
        .split("url(")
        .find_map(|part| part.strip_prefix("fonts/KaTeX_Main-Regular."))
        .and_then(|part| part.split(')').next())
        .map(|suffix| css_dir.join(format!("fonts/KaTeX_Main-Regular.{suffix}")))
        .expect("rewritten font url should be present");
    assert!(rewritten_font.exists());

    let custom_asset = project.join("theme/assets/css/temporary.css");
    std::fs::write(&custom_asset, "body { color: red; }\n").unwrap();
    build(&project, &[]).success();
    assert!(std::fs::read_dir(&css_dir).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("temporary.")
    }));

    std::fs::remove_file(&custom_asset).unwrap();
    build(&project, &[]).success();
    assert!(!std::fs::read_dir(&css_dir).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("temporary.")
    }));
}

#[test]
fn template_helpers_render_urls_and_markdown() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    let layout = project.join("theme/layouts/page.html");
    std::fs::write(
        &layout,
        r#"{% extends "layouts/base.html" %}
{% block content %}
<p class="helper-url">{{ url_for("about.md") }}</p>
<div class="helper-markdown">{{ markdown("**bold**") | safe }}</div>
{{ content | safe }}
{% endblock %}
"#,
    )
    .unwrap();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .arg("build")
        .assert()
        .success();

    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.contains(r#"<p class="helper-url">about.html</p>"#));
    assert!(html.contains("<strong>bold</strong>"));
}

#[test]
fn partial_keys_are_derived_from_relative_paths() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::create_dir_all(project.join("content/_partials/sections")).unwrap();
    std::fs::create_dir_all(project.join("content/_partials/nav")).unwrap();
    std::fs::write(project.join("content/_partials/sidebar.md"), "**Sidebar**").unwrap();
    std::fs::write(
        project.join("content/_partials/sections/sidebar.html"),
        "<p>Nested Sidebar</p>",
    )
    .unwrap();
    std::fs::write(
        project.join("content/_partials/nav/index.html"),
        "<p>Nav Index</p>",
    )
    .unwrap();

    std::fs::write(
        project.join("theme/layouts/page.html"),
        r#"{% extends "layouts/base.html" %}
{% block content %}
<aside class="plain-partial">{{ partials.sidebar | safe }}</aside>
<aside class="nested-partial">{{ partials["sections.sidebar"] | safe }}</aside>
<aside class="index-partial">{{ partials.nav | safe }}</aside>
{{ content | safe }}
{% endblock %}
"#,
    )
    .unwrap();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .arg("build")
        .assert()
        .success();

    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.contains(r#"<aside class="plain-partial"><p><strong>Sidebar</strong></p>"#));
    assert!(html.contains(r#"<aside class="nested-partial"><p>Nested Sidebar</p>"#));
    assert!(html.contains(r#"<aside class="index-partial"><p>Nav Index</p>"#));
}

#[test]
fn duplicate_partial_keys_are_errors() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::create_dir_all(project.join("content/_partials/nav")).unwrap();
    std::fs::write(project.join("content/_partials/nav.html"), "<p>Nav</p>").unwrap();
    std::fs::write(
        project.join("content/_partials/nav/index.html"),
        "<p>Nav index</p>",
    )
    .unwrap();

    build(&project, &[])
        .failure()
        .stderr(predicate::str::contains("duplicate partial key `nav`"))
        .stderr(predicate::str::contains("nav.html"))
        .stderr(predicate::str::contains("nav/index.html"));
}

#[test]
fn unused_partials_warn_on_full_and_cached_builds() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("content/_partials/unused.html"),
        "<p>Unused</p>",
    )
    .unwrap();

    build(&project, &["--profile"])
        .success()
        .stderr(predicate::str::contains(
            "warning: _partials: unused partial: unused",
        ))
        .stdout(predicate::str::contains("warnings: 1"));

    build(&project, &["--profile"])
        .success()
        .stderr(predicate::str::contains(
            "warning: _partials: unused partial: unused",
        ))
        .stdout(predicate::str::contains("pages rendered: 0"))
        .stdout(predicate::str::contains("warnings: 1"));
}

#[test]
fn unknown_front_matter_fields_warn() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("content/index.md"),
        "---\ntitle: Home\nkind: post\n---\n# Home\n",
    )
    .unwrap();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .args(["build", "--profile", "--no-cache"])
        .assert()
        .success()
        .stderr(predicate::str::contains(
            "warning: index.md:3:1: unknown front matter field: kind",
        ))
        .stderr(predicate::str::contains("  3 | kind: post"))
        .stderr(predicate::str::contains(
            "help: remove `kind` or model it in page content/templates",
        ))
        .stdout(predicate::str::contains("warnings: 1"))
        .stdout(predicate::str::contains("template load:"))
        .stdout(predicate::str::contains("markdown render:"))
        .stdout(predicate::str::contains("katex render:"))
        .stdout(predicate::str::contains("code highlighting:"))
        .stdout(predicate::str::contains("layout render:"));
}

#[test]
fn invalid_config_and_front_matter_include_source_locations() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    let config = project.join("blogx.toml");
    let valid_config = std::fs::read_to_string(&config).unwrap();
    std::fs::write(&config, "[site]\ntitle = ").unwrap();
    build(&project, &[])
        .failure()
        .stderr(predicate::str::contains("failed to parse blogx.toml:2:9"))
        .stderr(predicate::str::contains(
            "TOML parse error at line 2, column 9",
        ));

    std::fs::write(&config, valid_config).unwrap();
    std::fs::write(
        project.join("content/index.md"),
        "---\ntitle: [\n---\n# Bad\n",
    )
    .unwrap();
    build(&project, &[])
        .failure()
        .stderr(predicate::str::contains(
            "failed to parse front matter in index.md:3:1",
        ))
        .stderr(predicate::str::contains("  3 | ---"))
        .stderr(predicate::str::contains("^"));
}

#[test]
fn markdown_features_and_image_alt_warnings_are_rendered() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("content/index.md"),
        r#"# Features

| Name | Value |
| ---- | ----- |
| one  | two   |

- [x] done

Footnote here.[^1]

![Logo](logo.png)

![](missing-alt.png)

[^1]: Footnote body.
"#,
    )
    .unwrap();

    build(&project, &["--profile", "--no-cache"])
        .success()
        .stderr(predicate::str::contains(
            "warning: index.md:13:1: image is missing alt text",
        ))
        .stderr(predicate::str::contains("  13 | ![](missing-alt.png)"))
        .stderr(predicate::str::contains(
            "help: write descriptive text inside the image brackets",
        ))
        .stdout(predicate::str::contains("warnings: 1"));

    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.contains(r#"id="features""#));
    assert!(html.contains(r#">Features</h1>"#));
    assert!(html.contains("<table>"));
    assert!(html.contains(r#"type="checkbox""#));
    assert!(html.contains(r#"alt="Logo""#));
    assert!(html.contains("footnote"));
}

#[test]
fn code_and_katex_options_are_applied() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    let config = project.join("blogx.toml");
    let raw = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        raw.replace("line_numbers = true", "line_numbers = false")
            .replace("throw_on_error = true", "throw_on_error = false"),
    )
    .unwrap();
    std::fs::write(
        project.join("content/index.md"),
        "# Config\n\n$\\badcommand{$\n\n```rust\nfn main() {}\n```\n",
    )
    .unwrap();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .arg("build")
        .assert()
        .success();

    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(!html.contains("line-number"));
    assert!(html.contains("katex-error") || html.contains("badcommand"));
}

#[test]
fn all_katex_delimiters_render_at_build_time() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("content/index.md"),
        "# Math\n\nInline dollar $x + y$ and paren \\(a + b\\).\n\n$$\nc + d\n$$\n\n\\[\ne + f\n\\]\n",
    )
    .unwrap();

    build(&project, &[]).success();

    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.matches("class=\"katex").count() >= 4);
    assert!(html.contains("annotation encoding=\"application/x-tex\""));
    assert!(html.contains(r#"<script src="/assets/theme/js/copy-tex.min.js" defer></script>"#));
    assert!(!html.contains("$$"));
    assert!(!html.contains("\\("));
    assert!(!html.contains("\\["));
}

#[test]
fn katex_matrix_environments_render_at_build_time() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("content/index.md"),
        "# Matrix Math\n\n$$\n\\begin{vmatrix} a & b \\\\ c & d \\end{vmatrix}\n$$\n\n$$\n\\begin{pmatrix} x & y \\\\ z & w \\end{pmatrix}\n$$\n",
    )
    .unwrap();

    build(&project, &[]).success();

    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.matches("class=\"katex").count() >= 2);
    assert!(!html.contains("katex-error"));
    assert!(html.matches("<mtable").count() >= 2);
    assert!(html.contains("fence=\"true\">∣</mo>"));
    assert!(html.contains("<mo fence=\"true\">(</mo>"));
}

#[test]
fn katex_expression_cache_persists_across_build_processes() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("content/index.md"),
        "# Math\n\nInline $x + y$ and block:\n\n$$\na^2 + b^2 = c^2\n$$\n",
    )
    .unwrap();

    build(&project, &["--no-cache"]).success();
    let cache_path = project.join(".blogx/cache/katex.json");
    assert!(cache_path.exists());
    let cache = std::fs::read_to_string(cache_path).unwrap();
    assert!(cache.contains("x + y"));
    assert!(cache.contains("a^2 + b^2 = c^2"));

    build(&project, &["--profile", "--no-cache"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 3"))
        .stdout(predicate::str::contains("katex render: 0ms"));
}

#[test]
fn math_preprocessing_skips_inline_and_fenced_code() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("content/index.md"),
        "# Code dollars\n\nInline code `$HOME` remains code.\n\n```sh\necho \"$HOME\"\n```\n\nReal math $x + y$ renders.\n",
    )
    .unwrap();

    build(&project, &[]).success();

    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.contains("$HOME"));
    assert!(html.contains("echo"));
    assert!(html.contains("class=\"katex"));
}

#[test]
fn source_html_files_are_copied_without_template_rendering() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("content/raw.html"),
        "<!doctype html><title>{{ site.title }}</title><p>Raw</p>",
    )
    .unwrap();

    build(&project, &[]).success();

    let raw = std::fs::read_to_string(project.join("public/raw.html")).unwrap();
    assert_eq!(
        raw,
        "<!doctype html><title>{{ site.title }}</title><p>Raw</p>"
    );
}

#[test]
fn private_dirs_drafts_static_assets_and_clean_flag_work() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::create_dir_all(project.join("content/_drafts")).unwrap();
    std::fs::write(project.join("content/_drafts/hidden.md"), "# Hidden\n").unwrap();
    std::fs::write(
        project.join("content/draft.md"),
        "---\ndraft: true\n---\n# Draft\n",
    )
    .unwrap();
    std::fs::write(project.join("content/file.pdf"), b"fake pdf").unwrap();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .arg("build")
        .assert()
        .success();

    assert!(!project.join("public/_drafts/hidden.html").exists());
    assert!(!project.join("public/draft.html").exists());
    assert_eq!(
        std::fs::read(project.join("public/file.pdf")).unwrap(),
        b"fake pdf"
    );

    std::fs::write(project.join("public/stale.txt"), "stale").unwrap();

    Command::cargo_bin("blogx")
        .unwrap()
        .current_dir(&project)
        .args(["build", "--clean"])
        .assert()
        .success();

    assert!(!project.join("public/stale.txt").exists());
}

#[test]
fn images_are_copied_without_derived_formats_or_srcset() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::create_dir_all(project.join("content/img")).unwrap();
    std::fs::write(project.join("content/img/photo.jpg"), b"fake jpg").unwrap();
    std::fs::write(
        project.join("content/index.md"),
        "# Image\n\n![Photo](img/photo.jpg)\n",
    )
    .unwrap();

    build(&project, &[]).success();

    assert_eq!(
        std::fs::read(project.join("public/img/photo.jpg")).unwrap(),
        b"fake jpg"
    );
    assert!(!project.join("public/img/photo.webp").exists());
    assert!(!project.join("public/img/photo.avif").exists());
    assert!(!project.join("public/img/photo-320.jpg").exists());
    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.contains(r#"src="img/photo.jpg""#));
    assert!(html.contains(r#"alt="Photo""#));
    assert!(!html.contains("srcset="));
}

#[test]
fn legacy_protect_markdown_files_are_errors_not_public_pages() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("content/secret.protect.md"),
        "# Secret\n\nThis must not be silently published.\n",
    )
    .unwrap();

    build(&project, &[])
        .failure()
        .stderr(predicate::str::contains(
            "legacy protected page is not supported",
        ))
        .stderr(predicate::str::contains("secret.protect.md"));
    assert!(!project.join("public/secret.protect.html").exists());
    assert!(!project.join("public/secret.html").exists());
}

#[test]
fn static_assets_cache_no_cache_and_deleted_cleanup_work() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::create_dir_all(project.join("content/files")).unwrap();
    std::fs::write(project.join("content/files/data.txt"), "v1").unwrap();

    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("assets copied: 1"));
    assert_eq!(
        std::fs::read_to_string(project.join("public/files/data.txt")).unwrap(),
        "v1"
    );

    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("assets cached: 1"));

    build(&project, &["--profile", "--no-cache"])
        .success()
        .stdout(predicate::str::contains("assets copied: 1"))
        .stdout(predicate::str::contains(
            "cache mode: disabled by --no-cache",
        ));

    std::fs::remove_file(project.join("content/files/data.txt")).unwrap();
    build(&project, &["--profile"]).success();
    assert!(!project.join("public/files/data.txt").exists());
}

#[test]
fn partial_layout_and_config_changes_invalidate_rendered_pages() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 3"));
    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 0"));

    std::fs::write(
        project.join("content/_partials/header.md"),
        "[Home](index.md)\n",
    )
    .unwrap();
    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 3"));

    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 0"));
    let layout = project.join("theme/layouts/page.html");
    let raw_layout = std::fs::read_to_string(&layout).unwrap();
    std::fs::write(
        &layout,
        raw_layout.replace(
            "</article>",
            r#"<p class="layout-marker">Layout changed</p>
  </article>"#,
        ),
    )
    .unwrap();
    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 3"));
    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.contains("Layout changed"));

    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 0"));
    let head = project.join("theme/partials/head.html");
    let raw_head = std::fs::read_to_string(&head).unwrap();
    std::fs::write(
        &head,
        format!("{raw_head}\n<meta name=\"theme-partial-test\" content=\"changed\">\n"),
    )
    .unwrap();
    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 3"));
    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.contains("theme-partial-test"));

    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 0"));
    let config = project.join("blogx.toml");
    let raw_config = std::fs::read_to_string(&config).unwrap();
    std::fs::write(
        &config,
        raw_config.replace("title = \"site\"", "title = \"Changed Site\""),
    )
    .unwrap();
    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 3"));
    let html = std::fs::read_to_string(project.join("public/index.html")).unwrap();
    assert!(html.contains("Changed Site"));
}

#[test]
fn large_fixture_profiles_cached_and_single_page_rebuilds() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::create_dir_all(project.join("content/notes")).unwrap();
    for index in 0..1000 {
        std::fs::write(
            project.join(format!("content/notes/page-{index:03}.md")),
            format!(
                r#"---
title: "Page {index:03}"
---

# Page {index:03}

Inline math $a^2 + b^2 = c^2$ and display math:

$$
\int_0^1 x^2 dx
$$

```rust
fn page_{index:03}() -> usize {{
    {index}
}}
```
"#
            ),
        )
        .unwrap();
    }

    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 1003"))
        .stdout(predicate::str::contains("source scan:"))
        .stdout(predicate::str::contains("template load:"))
        .stdout(predicate::str::contains("markdown render:"))
        .stdout(predicate::str::contains("katex render:"))
        .stdout(predicate::str::contains("code highlighting:"))
        .stdout(predicate::str::contains("total:"));

    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 0"))
        .stdout(predicate::str::contains("pages cached: 1003"));

    std::fs::write(
        project.join("content/notes/page-042.md"),
        r#"---
title: "Page 042"
---

# Page 042

Changed content with $x + y$.

```rust
fn changed() {}
```
"#,
    )
    .unwrap();

    build(&project, &["--profile"])
        .success()
        .stdout(predicate::str::contains("pages rendered: 1"))
        .stdout(predicate::str::contains("pages cached: 1002"));
}

#[test]
fn template_errors_include_template_name_and_line() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("theme/layouts/page.html"),
        "{% extends \"layouts/base.html\" %}\n{% block content %}\n{{ content | safe }\n{% endblock %}\n",
    )
    .unwrap();

    build(&project, &[])
        .failure()
        .stderr(predicate::str::contains(
            "failed to load layout page.html at theme/layouts/page.html:3:19",
        ))
        .stderr(predicate::str::contains("{{ content | safe }"));
}

#[test]
fn missing_selected_layout_is_an_error() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("content/index.md"),
        "---\nlayout: missing.html\n---\n# Missing layout\n",
    )
    .unwrap();

    build(&project, &[])
        .failure()
        .stderr(predicate::str::contains(
            "failed to load layout missing.html",
        ))
        .stderr(predicate::str::contains(
            "template \"layouts/missing.html\" does not exist",
        ));
}

#[test]
fn missing_included_template_and_helper_errors_fail_build() {
    let temp = TempDir::new().unwrap();
    let project = temp.path().join("site");

    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&project)
        .assert()
        .success();

    std::fs::write(
        project.join("theme/layouts/page.html"),
        "{% include \"partials/does-not-exist.html\" %}\n{{ content | safe }}\n",
    )
    .unwrap();
    build(&project, &[])
        .failure()
        .stderr(predicate::str::contains(
            "failed to render layout page.html",
        ))
        .stderr(predicate::str::contains("partials/does-not-exist.html"));

    let helper_project = temp.path().join("site-helper");
    Command::cargo_bin("blogx")
        .unwrap()
        .arg("init")
        .arg(&helper_project)
        .assert()
        .success();
    std::fs::write(
        helper_project.join("theme/layouts/page.html"),
        "{{ markdown('$\\\\badcommand{$') | safe }}\n{{ content | safe }}\n",
    )
    .unwrap();
    build(&helper_project, &[])
        .failure()
        .stderr(predicate::str::contains("markdown helper failed"));
}
