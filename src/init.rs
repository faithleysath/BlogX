use std::{
    cmp::Reverse,
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use include_dir::{Dir, include_dir};
use walkdir::WalkDir;

static DEFAULT_THEME: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/default-theme");

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct ThemeSyncStats {
    pub files_copied: usize,
    pub files_pruned: usize,
    pub dirs_pruned: usize,
}

#[derive(Debug, Default)]
struct ThemeManifest {
    files: HashSet<PathBuf>,
    dirs: HashSet<PathBuf>,
}

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

    sync_default_theme(&name.join("theme"), false)?;
    println!("initialized {}", name.display());
    Ok(())
}

pub fn copy_default_theme(dest: &Path) -> Result<()> {
    sync_default_theme(dest, false).map(|_| ())
}

pub fn sync_default_theme(dest: &Path, prune: bool) -> Result<ThemeSyncStats> {
    fs::create_dir_all(dest).with_context(|| format!("failed to create {}", dest.display()))?;

    let root = DEFAULT_THEME.path().to_path_buf();
    let mut manifest = ThemeManifest::default();
    let files_copied = copy_included_dir_with_root(&DEFAULT_THEME, &root, dest, &mut manifest)?;
    let (files_pruned, dirs_pruned) = if prune {
        prune_extra_theme_entries(dest, &manifest)?
    } else {
        (0, 0)
    };

    Ok(ThemeSyncStats {
        files_copied,
        files_pruned,
        dirs_pruned,
    })
}

fn copy_included_dir_with_root(
    dir: &Dir<'_>,
    root: &Path,
    dest: &Path,
    manifest: &mut ThemeManifest,
) -> Result<usize> {
    let mut copied = 0;

    for child in dir.dirs() {
        let relative = child
            .path()
            .strip_prefix(root)
            .unwrap_or_else(|_| child.path());
        if !relative.as_os_str().is_empty() {
            manifest.dirs.insert(relative.to_path_buf());
            fs::create_dir_all(dest.join(relative))
                .with_context(|| format!("failed to create {}", dest.join(relative).display()))?;
        }
        copied += copy_included_dir_with_root(child, root, dest, manifest)?;
    }

    for file in dir.files() {
        let relative = file
            .path()
            .strip_prefix(root)
            .unwrap_or_else(|_| file.path());
        let path = dest.join(relative);
        manifest.files.insert(relative.to_path_buf());
        record_parent_dirs(relative, &mut manifest.dirs);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::write(&path, file.contents())
            .with_context(|| format!("failed to write {}", path.display()))?;
        copied += 1;
    }

    Ok(copied)
}

fn record_parent_dirs(path: &Path, dirs: &mut HashSet<PathBuf>) {
    let mut parent = path.parent();
    while let Some(dir) = parent {
        if dir.as_os_str().is_empty() {
            break;
        }
        dirs.insert(dir.to_path_buf());
        parent = dir.parent();
    }
}

fn prune_extra_theme_entries(dest: &Path, manifest: &ThemeManifest) -> Result<(usize, usize)> {
    let mut files_pruned = 0;
    let mut dirs = Vec::new();

    for entry in WalkDir::new(dest).min_depth(1).follow_links(false) {
        let entry = entry.with_context(|| format!("failed to read {}", dest.display()))?;
        let path = entry.path().to_path_buf();
        let relative = path
            .strip_prefix(dest)
            .with_context(|| format!("failed to inspect {}", path.display()))?
            .to_path_buf();

        if entry.file_type().is_dir() {
            dirs.push((entry.depth(), path, relative));
        } else if !manifest.files.contains(&relative) {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove {}", path.display()))?;
            files_pruned += 1;
        }
    }

    dirs.sort_by_key(|entry| Reverse(entry.0));
    let mut dirs_pruned = 0;
    for (_, path, relative) in dirs {
        if manifest.dirs.contains(&relative) {
            continue;
        }
        match fs::remove_dir(&path) {
            Ok(()) => dirs_pruned += 1,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) if err.kind() == io::ErrorKind::DirectoryNotEmpty => {}
            Err(err) => {
                return Err(err).with_context(|| format!("failed to remove {}", path.display()));
            }
        }
    }

    Ok((files_pruned, dirs_pruned))
}
