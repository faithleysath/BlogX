# BlogX

BlogX is an HTML+CSS-first static site compiler written in Rust.

It keeps the original mental model deliberately small: the file tree is the
website, one Markdown file is one page, and homepages, directories, navigation,
archives, tags, and category pages are maintained by the author as normal
content. BlogX renders pages, copies assets, applies templates, and generates
only machine-readable site infrastructure such as `sitemap.xml` and
`robots.txt`.

The Rust rewrite is a breaking rewrite. It does not preserve the old Python
package, the old string-replacement templates, the old `src/_global` layout, or
the old `.protect.md` behavior.

## Status

This branch contains the Rust rewrite described in
[`docs/rust-rewrite-decisions.md`](docs/rust-rewrite-decisions.md). The
implementation is usable for local builds, but the public compatibility story is
the new Rust model rather than migration-free reuse of old BlogX projects.

## Install

From this repository:

```bash
cargo install --path .
```

During development:

```bash
cargo run -- --help
cargo run -- init my-site
```

## Quick Start

```bash
blogx init my-site
cd my-site
blogx build --profile
blogx serve --host 127.0.0.1 --port 8000
```

The default project layout is:

```text
blogx.toml
content/
  index.md
  about.md
  notes/
    hello.md
  _partials/
    header.md
    footer.md
  _drafts/
    unfinished.md
theme/
  theme.toml
  layouts/
  partials/
  assets/
public/
```

`content/` is compiled into `public/`. Markdown files become pages. Non-Markdown
files are copied as static assets. Directories whose names start with `_` are
private inputs and are not published directly.

## Commands

```bash
blogx init <name>
blogx build
blogx build --clean
blogx build --no-cache
blogx build --jobs 8
blogx build --profile
blogx serve
blogx clean
```

The old `deploy` command is intentionally not part of the first Rust version.

## Content Model

Front matter is optional. The first version supports only page-level rendering
metadata:

```yaml
---
title: "About"
layout: "page.html"
draft: false
---
```

Supported URL modes:

- `html`: `about.md` outputs `about.html`
- `clean`: `about.md` outputs `about/index.html` and links to `about/`

BlogX rewrites only local links to Markdown pages, for example
`[About](about.md)`. External links, anchors, absolute URLs, and non-Markdown
links are left as written.

## Rendering

Markdown rendering is based on `comrak` and includes CommonMark, tables,
footnotes, task lists, heading anchors, fenced code blocks, and build-time math
rendering.

Math is rendered at build time with KaTeX-compatible HTML. The default theme
ships local KaTeX CSS and fonts, so browser-side JavaScript is not required for
formula display.

Code blocks are highlighted at build time with `syntect`. Line numbers are
static HTML and CSS, not client-side rendering.

## Themes

Themes use MiniJinja templates under `theme/layouts/`.

Templates receive explicit context objects:

- `site`
- `page`
- `content`
- `partials`
- `assets`
- `build`

Built-in helpers:

- `asset_url("css/main.css")`
- `url_for("about.md")`
- `markdown("**small fragment**")`

Content partials live in `content/_partials/`. Markdown partials are rendered to
HTML, HTML partials are passed through, and keys are derived deterministically
from their relative path without the extension. For example,
`content/_partials/sidebar.md` is exposed as `partials.sidebar`,
`content/_partials/sections/sidebar.md` is exposed as
`partials["sections.sidebar"]`, and `content/_partials/nav/index.html` is exposed
as `partials.nav`.

## No-JS Principle

The generated site must remain readable and navigable with JavaScript disabled.
The default theme uses HTML and CSS for reading, navigation, responsive layout,
math display, code highlighting, and line numbers. JavaScript is limited to
progressive enhancement such as copy buttons.

## Performance

BlogX uses a compiler-style build pipeline with parallel page rendering,
persistent cache state in `.blogx/cache/`, cached KaTeX expressions, incremental
static asset copies, and orphan output cleanup.

Use `blogx build --profile` to inspect scan, partial, Markdown, KaTeX, code
highlighting, layout, asset, cache, and total build timings.

## Development

```bash
cargo fmt --check
cargo check
cargo test
bash scripts/no-js-smoke.sh
bash scripts/browser-acceptance.sh
bash scripts/serve-watch-smoke.sh
```

Manual release acceptance is tracked in
[`docs/manual-acceptance.md`](docs/manual-acceptance.md).
