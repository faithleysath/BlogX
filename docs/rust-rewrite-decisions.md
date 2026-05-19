# BlogX Rust Rewrite Decisions

This document records the product and architecture decisions for the Rust
rewrite of BlogX. It is intentionally decision-complete: implementation should
follow these rules unless the document is explicitly revised.

## 1. Positioning

BlogX is an HTML+CSS-first static site compiler.

It compiles a manually maintained file tree into a static website. It is not a
CMS, not a blog information-architecture engine, and not a frontend application
framework.

The core promise is:

- The file tree defines the website.
- Markdown defines pages.
- Templates define presentation.
- HTML and CSS carry the core experience.
- JavaScript only provides progressive enhancement.

The Rust rewrite may introduce large breaking changes. It does not need to run
existing BlogX projects without migration, and it does not need to preserve the
old Python package, old template placeholders, old project layout, or old
`.protect.md` behavior.

## 2. Non-Goals

BlogX must not generate visible content structure for the user.

The following are out of scope for the core system:

- Automatically generated homepages.
- Automatically generated navigation.
- Automatically generated article lists.
- Automatically generated directories.
- Automatically generated archives.
- Tags and categories.
- A built-in comment system.
- A built-in search experience.
- Client-side rendering as a core path.
- Any requirement that JavaScript runs for normal reading.

Machine-readable site infrastructure is allowed when it does not create visible
content structure. `sitemap.xml` and `robots.txt` are allowed. RSS is deferred
because it introduces dates, summaries, and content collections.

## 3. Project Model

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
    footer.html
  _drafts/
    unfinished.md
theme/
  theme.toml
  layouts/
  partials/
  assets/
public/
```

Default paths:

- Source directory: `content/`
- Output directory: `public/`
- Theme directory: `theme/`
- Config file: `blogx.toml`
- Build cache: `.blogx/cache/`

These defaults may be configurable in `blogx.toml`, but the defaults should be
the primary documented path.

## 4. Domain Model

BlogX has only two source item kinds:

- `page`: a Markdown file that renders through a layout.
- `asset`: any non-Markdown file that is copied to output.

BlogX does not have `post`, `article`, `category`, `tag`, `archive`, or `index`
as domain concepts. Users can express those ideas manually through pages,
links, directories, and templates.

## 5. File Tree Rules

The source file tree maps to the output website.

- Markdown files generate pages.
- Non-Markdown files are copied as static assets.
- Source `.html` files are copied as static assets; they are not run through the
  template engine.
- Any directory whose name starts with `_` is private input and is not copied or
  rendered as a public page tree.
- `_partials/` is the conventional private directory for reusable content
  fragments.
- `_drafts/` is the conventional private directory for drafts.
- Files inside private directories may still be consumed by BlogX as inputs when
  they have a defined role, such as partials.

Draft rules:

- A page inside any `_` directory is not output.
- A page with front matter `draft: true` is not output.

## 6. URL Rules

URL shape is configurable at the site level.

Supported URL modes:

- `html`: `about.md` outputs `about.html`.
- `clean`: `about.md` outputs `about/index.html`.

Default URL mode:

- `html`

Markdown links are author-maintained. BlogX performs only one link transform:
local links to `.md` pages are rewritten to the target page URL according to the
configured URL mode.

Examples in `html` mode:

- `[About](about.md)` becomes `[About](about.html)` in output HTML.
- `[Note](notes/hello.md)` becomes `[Note](notes/hello.html)` in output HTML.

Examples in `clean` mode:

- `[About](about.md)` becomes `[About](about/)` in output HTML.
- `[Note](notes/hello.md)` becomes `[Note](notes/hello/)` in output HTML.

In `clean` mode, the filesystem output is still `about/index.html`, but public
links should use the directory URL form `about/`.

External links, anchors, absolute URLs, and non-Markdown links are not rewritten.

Broken local Markdown links should produce warnings by default, not hard errors,
unless they make output path resolution impossible.

## 7. Front Matter

Front matter is optional.

Pages without front matter must build successfully.

Supported front matter fields for the first Rust version:

```yaml
title: "Page title"
layout: "page.html"
draft: false
```

Semantics:

- `title` overrides the page title used by templates.
- `layout` overrides the configured default layout.
- `draft: true` skips output for the page.

Front matter must not imply that BlogX has an article/post model. It only
overrides page rendering behavior.

## 8. HTML+CSS-First Principle

The generated site must provide the core experience with only HTML and CSS.

Core no-JS requirements:

- Reading page content must work without JavaScript.
- Site navigation written by the user/theme must work without JavaScript.
- Manual directory/index pages must work without JavaScript.
- Responsive layout must work without JavaScript.
- Math rendering must work without JavaScript.
- Code blocks, syntax highlighting, and line numbers must work without
  JavaScript.
- Images must be readable and responsive without JavaScript.
- Page layout must not depend on client-side rendering.

JavaScript may provide only progressive enhancements, such as:

- Copy buttons for code blocks.
- Keyboard shortcuts.
- Image lightbox behavior.
- Small convenience interactions.

If JavaScript is disabled, blocked, or fails to load, the page must still be
readable and navigable.

## 9. Markdown Rendering

The Markdown implementation should be based on a Rust Markdown parser with AST
access, such as `comrak`.

Required Markdown capabilities:

- CommonMark-compatible core syntax.
- Tables.
- Footnotes.
- Task lists.
- Heading anchors.
- Fenced code blocks.
- Server-side math rendering.

Shortcodes, admonitions, galleries, and component syntax are not part of the
first Rust version. They can be reconsidered later, but the first version should
avoid turning the content layer into a component framework.

## 10. Math Rendering

Math must be rendered at build time.

The renderer should use a Rust KaTeX implementation and emit KaTeX HTML plus
local CSS. Browser-side JavaScript must not be required for formula display.

Supported delimiters:

- Inline math: `$...$`
- Inline math: `\(...\)`
- Block math: `$$...$$`
- Block math: `\[...\]`

KaTeX render results should be cacheable by expression text plus render mode
and renderer version.

## 11. Code Rendering

Code blocks must be rendered at build time.

Required behavior:

- Syntax highlighting is generated as static HTML/CSS.
- Line numbers are generated without JavaScript.
- Code remains readable without syntax-specific CSS.
- Copy buttons, if present, are progressive JavaScript enhancement only.

The implementation should use a Rust highlighter such as `syntect`.

## 12. Image Behavior

Images should remain simple static assets.

Required behavior:

- Images are copied as assets.
- Rendered images should be responsive by default in the official theme.
- `alt` text is preserved.
- Missing `alt` text should produce a warning by default.

The first Rust version should not implement image transcoding, thumbnail
generation, AVIF/WebP generation, responsive `srcset` generation, or automatic
lightbox behavior as a core feature.

## 13. Theme System

The old string-replacement template mechanism is removed.

The Rust rewrite should use a real template engine with Jinja-style syntax,
preferably MiniJinja.

The theme system should support:

- Layouts.
- Layout inheritance.
- Blocks.
- Includes.
- Partials.
- Theme assets.
- A small set of built-in helpers.
- Template errors with file path and line/column information.

The theme system must not generate navigation, indexes, tags, categories, or
article collections.

## 14. Theme Directory Layout

Official themes should use this structure:

```text
theme/
  theme.toml
  layouts/
    base.html
    page.html
  partials/
    head.html
    header.html
    nav.html
    footer.html
  assets/
    css/
      main.css
      syntax.css
      katex.css
    js/
      enhance.js
```

`layouts/page.html` may extend `layouts/base.html`.

Theme partials are included explicitly from templates. There are no hard-coded
`header`, `sidebar`, or `footer` slots.

## 15. Template Context

Templates should receive a small, explicit, stable context.

Context objects:

- `site`: site configuration from `blogx.toml`.
- `page`: current page metadata and resolved paths.
- `content`: rendered HTML for the current Markdown page.
- `partials`: rendered partial content from `content/_partials/`.
- `assets`: theme asset access object.
- `build`: BlogX version and build metadata.

Avoid broad top-level convenience variables such as `title`, `url`, or `date`.
Templates should use explicit paths such as `page.title` and `site.title`.

## 16. Template Helpers

The first Rust version should provide only a small helper set:

- `asset_url("css/main.css")`: returns the public URL for a theme asset.
- `url_for("about.md")`: returns the output URL for a source page path.
- `markdown("...")`: renders a small Markdown string in template context.

No plugin system, user-defined filters, or script-based template extensions are
included in the first Rust version.

## 17. Partials

Partials are reusable content fragments from the source tree.

Rules:

- The conventional location is `content/_partials/`.
- Markdown partials are rendered to HTML.
- HTML partials are made available as HTML.
- Partials are exposed under the `partials` template context.
- Templates decide which partials to use.
- BlogX does not assume fixed partial names.

Example:

```text
content/_partials/sidebar.md
```

Should be available to templates as a stable key such as:

```jinja
{{ partials.sidebar | safe }}
```

The exact naming policy should be deterministic and documented during
implementation.

## 18. Theme Assets

Theme assets are copied from:

```text
theme/assets/
```

to:

```text
public/assets/theme/
```

Templates should reference theme assets through helpers, not hard-coded output
paths.

The first Rust version should support optional fingerprinting for theme assets.

Fingerprinting behavior:

- It should be configurable.
- It should produce stable cache-friendly URLs.
- It should not require a JavaScript or Node-based asset pipeline.

The first Rust version should not perform bundling, minification, tree-shaking,
PostCSS processing, Tailwind processing, or Vite-style builds.

## 19. Default Theme Requirements

The official default theme must be modern but HTML+CSS-first.

Required theme properties:

- No external CDN dependency for core rendering.
- No external font dependency for core rendering.
- Local KaTeX CSS.
- Local syntax highlighting CSS.
- Semantic HTML.
- Skip link.
- Clear focus styles.
- Responsive layout.
- `prefers-reduced-motion` support.
- No JavaScript requirement for reading, navigation, math, code highlighting, or
  line numbers.
- Optional `defer` JavaScript for progressive enhancements only.

The default CSS should use modern native CSS:

- CSS custom properties.
- Cascade layers where useful.
- Media queries.
- Container queries where useful.

The default theme should not require Node, Tailwind, Vite, PostCSS, Sass, or
Less.

## 20. CLI

The Rust CLI should use `clap`.

Required commands:

```text
blogx init <name>
blogx build
blogx serve
blogx clean
```

Required build flags:

```text
blogx build --clean
blogx build --no-cache
blogx build --jobs <N>
blogx build --profile
```

Required serve flags:

```text
blogx serve --host 127.0.0.1
blogx serve --port 8000
blogx serve --jobs <N>
```

The old `deploy` command is not required for the first Rust version. If it is
reintroduced, it should generate Rust-based GitHub Actions and should not push
without an explicit flag.

## 21. Configuration

`blogx.toml` is the site configuration file.

It should cover:

- Site title.
- Source directory.
- Output directory.
- Theme directory.
- Default layout.
- URL mode.
- Markdown options.
- KaTeX options.
- Code highlighting options.
- Theme asset fingerprinting.
- Serve host and port defaults.
- Sitemap and robots settings.

Configuration defaults should allow a minimal project to build without a large
config file.

## 22. Build Pipeline

The build pipeline should be structured as a compiler pipeline:

1. Load configuration.
2. Load theme metadata and templates.
3. Discover source pages, assets, private inputs, and partials.
4. Resolve output paths and detect collisions.
5. Build dependency fingerprints.
6. Render partials.
7. Render Markdown pages.
8. Render pages through layouts.
9. Copy content assets.
10. Copy theme assets.
11. Generate allowed machine-readable site files.
12. Write cache state.
13. Report diagnostics and build summary.

The pipeline must make dependency relationships explicit enough to support
correct incremental builds.

## 23. Performance Model

Performance is a core requirement for the Rust rewrite.

Required capabilities:

- Parallel full builds.
- Persistent incremental cache.
- KaTeX expression cache.
- Static asset incremental copy.
- Orphan output cleanup.
- Watch-mode incremental rebuilds.

Recommended implementation tools:

- `rayon` for parallel page rendering.
- `walkdir` or `ignore` for filesystem walking.
- `notify` for file watching.

Cache location:

```text
.blogx/cache/
```

Cache keys should include:

- Source content hash.
- Config hash.
- Template hash.
- Partial hash.
- Renderer version.
- Markdown option hash.
- KaTeX option hash.
- Code highlighting option hash.
- Theme asset fingerprinting settings.

Cache invalidation rules:

- Editing one page rebuilds that page.
- Editing one copied asset copies that asset.
- Deleting a source page removes its output.
- Deleting a source asset removes its output.
- Editing a partial rebuilds pages that depend on partial context.
- Editing a layout or included template rebuilds pages that depend on it.
- Editing global rendering config invalidates affected rendered outputs.
- Renderer version changes invalidate rendered outputs.

If dependency tracking is uncertain, correctness wins over minimal rebuilds.

## 24. Serve Mode

`blogx serve` should:

- Perform an initial build.
- Serve the output directory as static files.
- Watch `content/`, `theme/`, and `blogx.toml`.
- Reuse the same incremental build logic as `blogx build`.
- Report what changed and what was rebuilt.
- Optionally provide live reload as progressive enhancement.

Live reload must not become a requirement for normal generated pages.

## 25. Diagnostics

Diagnostics should feel like compiler diagnostics.

Where possible, errors and warnings should include:

- Severity.
- File path.
- Line and column.
- Source snippet.
- Explanation.
- Suggested fix.

Warnings by default:

- Broken local Markdown link.
- Missing image `alt`.
- Unused partials, if this can be detected reliably.
- Unknown front matter fields, if strict mode is not enabled.

Errors:

- Invalid `blogx.toml`.
- Invalid front matter syntax.
- Duplicate output path.
- Missing selected layout.
- Template syntax error.
- Missing explicitly included template.
- Render failure that prevents page output.
- Asset copy failure.

## 26. Profiling

`blogx build --profile` should print a compact profile report.

The report should include at least:

- Source scan time.
- Template load time.
- Partial render time.
- Markdown parse/render time.
- KaTeX render time.
- Code highlighting time.
- Layout render time.
- Asset copy time.
- Site infrastructure generation time.
- Cache hit/miss counts.
- Total build time.

This is intended for diagnosing large-blog build performance.

## 27. Site Infrastructure

The first Rust version may generate:

- `sitemap.xml`
- `robots.txt`

These outputs are allowed because they are machine-readable site infrastructure,
not visible content structure.

The first Rust version should not generate:

- RSS feeds.
- JSON feeds.
- Search indexes.
- Visible archives.
- Visible tag pages.
- Visible category pages.
- Visible navigation pages.

## 28. Accessibility

The default theme and renderer should provide baseline accessibility.

Required:

- Semantic HTML in the default theme.
- Skip link.
- Visible focus styles.
- Respect `prefers-reduced-motion`.
- Preserve image `alt`.
- Warn for missing image `alt`.
- Do not require JavaScript for navigation or content access.

Strict accessibility failures are not required in the first version. Warnings
are preferred unless output would be broken.

## 29. Protection Feature

The old `.protect.md` feature is removed from the first Rust rewrite.

Reasons:

- It depends on old filename conventions.
- It is not part of the core file-tree compiler model.
- The old base64 payload is not real encryption.
- It complicates the no-JS-first model.

Protected pages may be redesigned later as a separate feature, but the first
Rust version should not carry the old behavior forward.

## 30. Distribution

Primary distribution is a Rust binary.

Supported paths:

- `cargo install`
- Prebuilt release binaries

The first Rust rewrite does not need to preserve `pip install blogx` as the
primary installation path.

## 31. Testing Requirements

Unit tests should cover:

- Path mapping.
- URL modes.
- Private directory skipping.
- Draft skipping.
- Markdown link rewriting.
- Image attribute preservation and missing-alt warnings.
- Front matter parsing.
- Partial key derivation.
- Template helper behavior.
- Cache key construction.

Rendering tests should cover:

- Server-side KaTeX output.
- Syntax highlighting output.
- Line number output.
- Heading anchor output.
- Tables, footnotes, and task lists.
- No-JS readability of generated HTML.

Integration tests should cover:

- `blogx init`.
- First full build.
- Cached rebuild.
- `--clean`.
- `--no-cache`.
- Static asset copy.
- Theme asset copy.
- Sitemap and robots generation.
- Deleted source cleanup.

Incremental tests should cover:

- Editing one page rebuilds only that page when safe.
- Editing one asset copies only that asset.
- Editing a partial rebuilds dependent pages.
- Editing a layout rebuilds dependent pages.
- Editing config invalidates affected outputs.
- Deleting a source file removes orphaned output.

Performance tests should cover:

- A large fixture with hundreds or thousands of Markdown files.
- Pages containing formulas.
- Pages containing highlighted code blocks.
- First full build timing.
- Cached rebuild timing.
- Single-page rebuild timing.

Manual acceptance should include:

- Browser test with JavaScript disabled.
- Responsive layout check.
- Math display check.
- Code highlighting and line number check.
- Navigation check.
- Image rendering check.
- `serve` watch rebuild check.

## 32. First-Version Deferrals

The following should not be implemented in the first Rust rewrite unless this
document is revised:

- RSS or JSON feed generation.
- Search indexes.
- Visible auto-generated archive pages.
- Visible auto-generated tag/category pages.
- Shortcodes.
- Admonition syntax.
- Gallery syntax.
- Plugin system.
- User-defined template filters/functions.
- Node-based asset pipeline.
- CSS/JS bundling.
- CSS/JS minification.
- Image transcoding.
- Responsive image generation.
- Protected pages.
- Python package compatibility.
- Direct old-project compatibility.
