use std::{fs, path::Path};

use anyhow::{Context, Result, bail};
use include_dir::{Dir, include_dir};

static DEFAULT_THEME: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/default-theme");

pub fn init_project(name: &Path) -> Result<()> {
    if name.exists() {
        bail!("project path already exists: {}", name.display());
    }

    fs::create_dir_all(name.join("content/notes"))
        .with_context(|| format!("failed to create {}", name.display()))?;
    fs::create_dir_all(name.join("content/_partials"))
        .with_context(|| format!("failed to create {}", name.display()))?;
    fs::create_dir_all(name.join("content/_drafts"))
        .with_context(|| format!("failed to create {}", name.display()))?;
    fs::create_dir_all(name.join("theme"))
        .with_context(|| format!("failed to create theme for {}", name.display()))?;

    fs::write(
        name.join("blogx.toml"),
        format!(
            r##"[site]
title = "{}"
base_url = "/"

[paths]
content = "content"
output = "public"
theme = "theme"
cache = ".blogx/cache"

[theme]
default_layout = "page.html"
url_mode = "html"

[markdown]
heading_anchors = true
unsafe_html = true

[katex]
throw_on_error = true
error_color = "#cc0000"
trust = false

[code]
theme = "base16-ocean.dark"
line_numbers = true

[assets]
fingerprint = false

[serve]
host = "127.0.0.1"
port = 8000

[site_infra]
sitemap = true
robots = true
"##,
            name.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("My Blog")
        ),
    )
    .context("failed to write blogx.toml")?;

    fs::write(
        name.join("content/index.md"),
        r#"---
title: Home
---

# Welcome

This site is built with BlogX.

- [About](about.md)
"#,
    )
    .context("failed to write content/index.md")?;

    fs::write(
        name.join("content/about.md"),
        r#"---
title: About
---

# About

Edit `content/about.md` to introduce the site.
"#,
    )
    .context("failed to write content/about.md")?;

    fs::write(
        name.join("content/notes/hello.md"),
        r#"---
title: Hello
---

# Hello

This is a manually linked note. Add it to `content/index.md` when you want it in navigation.
"#,
    )
    .context("failed to write content/notes/hello.md")?;

    fs::write(
        name.join("content/_drafts/unfinished.md"),
        r#"---
title: Unfinished
draft: true
---

# Unfinished

Drafts stay private until moved out of `_drafts/` or marked `draft: false`.
"#,
    )
    .context("failed to write content/_drafts/unfinished.md")?;

    fs::write(
        name.join("content/_partials/header.md"),
        r#"[Home](index.md) · [About](about.md)
"#,
    )
    .context("failed to write content/_partials/header.md")?;

    fs::write(
        name.join("content/_partials/footer.md"),
        r#"Built with BlogX.
"#,
    )
    .context("failed to write content/_partials/footer.md")?;

    fs::write(
        name.join("content/_partials/sidebar.md"),
        r#"## Notes

- [Hello](notes/hello.md)
- Edit `content/_partials/sidebar.md`
"#,
    )
    .context("failed to write content/_partials/sidebar.md")?;

    copy_included_dir(&DEFAULT_THEME, &name.join("theme"))?;
    println!("initialized {}", name.display());
    Ok(())
}

pub fn copy_default_theme(dest: &Path) -> Result<()> {
    copy_included_dir(&DEFAULT_THEME, dest)
}

fn copy_included_dir(dir: &Dir<'_>, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest).with_context(|| format!("failed to create {}", dest.display()))?;
    let root = dir.path().to_path_buf();

    for file in dir.files() {
        let relative = file
            .path()
            .strip_prefix(&root)
            .unwrap_or_else(|_| file.path());
        let path = dest.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&path, file.contents())
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    for child in dir.dirs() {
        copy_included_dir_with_root(child, &root, dest)?;
    }

    Ok(())
}

fn copy_included_dir_with_root(dir: &Dir<'_>, root: &Path, dest: &Path) -> Result<()> {
    for file in dir.files() {
        let relative = file
            .path()
            .strip_prefix(root)
            .unwrap_or_else(|_| file.path());
        let path = dest.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&path, file.contents())
            .with_context(|| format!("failed to write {}", path.display()))?;
    }

    for child in dir.dirs() {
        copy_included_dir_with_root(child, root, dest)?;
    }

    Ok(())
}
