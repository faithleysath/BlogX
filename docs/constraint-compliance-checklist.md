# BlogX Constraint Compliance Checklist

This checklist tracks the implementation target from
`docs/rust-rewrite-decisions.md`.

## Implemented Follow-Ups

- [x] URI-scheme links are treated as non-local links before Markdown page link
  rewriting.
- [x] `robots.txt` omits `Sitemap: /sitemap.xml` when sitemap generation is
  disabled.
- [x] Legacy `*.protect.md` and `*.protect.markdown` pages fail the build
  instead of being silently published.
- [x] Content partial key collisions fail the build with both source paths.
- [x] Fingerprinted theme assets rewrite CSS `url(...)` references, including
  local KaTeX font URLs.
- [x] Deleted theme assets are removed from `public/assets/theme/`.
- [x] Cached pages replay Markdown diagnostics for broken local Markdown links,
  unknown front matter fields, and missing image `alt`.
- [x] KaTeX expression cache is persisted under `.blogx/cache/katex.json`.
- [x] Regression tests confirm images are copied without generated WebP, AVIF,
  thumbnail, or `srcset` outputs.
- [x] Manual acceptance includes fingerprinted no-JS rendering, theme asset
  deletion, protected-page migration, cached diagnostics, sitemap/robots
  consistency, and partial key collision checks.
- [x] Unused content partials are reliably detected by tracking actual template
  access to `partials` and warning after all pages are rendered or loaded from
  cache.
- [x] Static accessibility smoke checks semantic landmarks, skip link, focus
  styles, reduced-motion CSS, responsive images, non-empty image `alt`, and
  no-JS core rendering markers.
- [x] Performance integration coverage uses a 1000-page fixture with math and
  highlighted code, then checks first full build, cached rebuild, and single-page
  rebuild profile output.
- [x] Release artifact smoke checks the release workflow matrix, archive names,
  packaging commands, release upload configuration, and npm publish step.

## External Release Verification

- [ ] After a `v*` tag is pushed, verify the GitHub release contains the four
  workflow-built archives: Linux x86_64 glibc, Linux aarch64 glibc, macOS
  x86_64, and macOS aarch64. The Rust KaTeX binding uses its default QuickJS
  backend, so Windows and musl release targets are intentionally excluded from
  the supported prebuilt matrix. Also verify `@faithleysath/blogx` was published
  to npm. This depends on the external GitHub release run and cannot be proven
  from a local working tree before a tag exists.
