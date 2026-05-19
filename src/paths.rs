use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};

use crate::config::UrlMode;

pub fn is_private_path(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(name) => name.to_string_lossy().starts_with('_'),
        _ => false,
    })
}

pub fn is_markdown(path: &Path) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("md") || ext.eq_ignore_ascii_case("markdown"))
}

pub fn page_output_path(relative: &Path, mode: UrlMode) -> Result<PathBuf> {
    if !is_markdown(relative) {
        bail!("page source is not Markdown: {}", relative.display());
    }

    let stemmed = relative.with_extension("");
    Ok(match mode {
        UrlMode::Html => stemmed.with_extension("html"),
        UrlMode::Clean => {
            if stemmed.file_name().is_some_and(|name| name == "index") {
                stemmed.with_extension("html")
            } else {
                stemmed.join("index.html")
            }
        }
    })
}

pub fn page_public_url(relative: &Path, mode: UrlMode) -> Result<String> {
    let output = page_output_path(relative, mode)?;
    Ok(match mode {
        UrlMode::Html => normalize_url_path(&output),
        UrlMode::Clean => {
            if output.file_name().is_some_and(|name| name == "index.html") {
                let parent = output.parent().unwrap_or_else(|| Path::new(""));
                if parent.as_os_str().is_empty() {
                    "index.html".to_string()
                } else {
                    format!("{}/", normalize_url_path(parent))
                }
            } else {
                normalize_url_path(&output)
            }
        }
    })
}

pub fn normalize_url_path(path: &Path) -> String {
    path.components()
        .filter_map(|component| match component {
            Component::Normal(value) => Some(value.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub fn ensure_relative(path: &Path) -> Result<()> {
    if path.is_absolute() {
        bail!("path must be relative: {}", path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_html_urls() {
        assert_eq!(
            page_output_path(Path::new("notes/hello.md"), UrlMode::Html).unwrap(),
            PathBuf::from("notes/hello.html")
        );
        assert_eq!(
            page_public_url(Path::new("notes/hello.md"), UrlMode::Html).unwrap(),
            "notes/hello.html"
        );
    }

    #[test]
    fn maps_clean_urls() {
        assert_eq!(
            page_output_path(Path::new("notes/hello.md"), UrlMode::Clean).unwrap(),
            PathBuf::from("notes/hello/index.html")
        );
        assert_eq!(
            page_public_url(Path::new("notes/hello.md"), UrlMode::Clean).unwrap(),
            "notes/hello/"
        );
    }

    #[test]
    fn detects_private_paths() {
        assert!(is_private_path(Path::new("notes/_drafts/a.md")));
        assert!(!is_private_path(Path::new("notes/drafts/a.md")));
    }
}
