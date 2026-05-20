# BlogX Manual Acceptance

Use this checklist before treating the Rust rewrite as release-ready.

Automated smoke:

```bash
cargo build
cargo fmt --check
cargo test --all-targets
bash scripts/no-js-smoke.sh
bash scripts/accessibility-smoke.sh
bash scripts/browser-acceptance.sh
bash scripts/serve-watch-smoke.sh
bash scripts/release-artifact-smoke.sh
```

Manual browser checks:

- Build a test site with `blogx init`, add pages containing navigation links,
  images, KaTeX math, and fenced code blocks, then run `blogx build`.
- Open `public/index.html` or serve the site with `blogx serve`. The
  `scripts/browser-acceptance.sh` script performs this path with Firefox
  headless, JavaScript disabled, and desktop/mobile screenshots when Firefox is
  installed.
- Disable JavaScript in the browser and confirm reading, navigation, math,
  code highlighting, line numbers, and image rendering still work.
- Check narrow and wide viewport widths for readable responsive layout.
- Run `scripts/accessibility-smoke.sh` and confirm the generated default-theme
  HTML includes semantic landmarks, a skip link, visible focus styles,
  `prefers-reduced-motion`, responsive images, and non-empty image `alt`.
- Repeat the no-JS checks with `assets.fingerprint = true` and confirm local
  KaTeX fonts still load from `public/assets/theme/css/fonts/`.
- Start `blogx serve`, edit `content/`, `theme/`, and `blogx.toml`, and confirm
  rebuild output reports changed paths and updates generated pages.
- Delete a file under `theme/assets/` and confirm the old file is removed from
  `public/assets/theme/` after rebuild.

JavaScript acceptance:

- `assets/theme/js/enhance.js` may add convenience behavior such as copy
  buttons.
- Removing or blocking that script must not remove page content, navigation,
  math rendering, code highlighting, line numbers, or images.

Migration and diagnostics acceptance:

- Add `content/secret.protect.md` and confirm the build fails instead of
  silently publishing it.
- Add an unknown front matter field, a broken local Markdown link, and an image
  without `alt`; run two cached builds and confirm the warnings are reported on
  both builds.
- Disable `site_infra.sitemap` while keeping `site_infra.robots` enabled and
  confirm `robots.txt` does not reference a missing sitemap.
- Add conflicting partials such as `content/_partials/nav.html` and
  `content/_partials/nav/index.html`; confirm the build reports a duplicate
  partial key.
- Add an unused file under `content/_partials/`; run two cached builds and
  confirm the unused partial warning appears on both builds.

Performance and distribution acceptance:

- Run `cargo test --test cli_build large_fixture_profiles_cached_and_single_page_rebuilds`
  and confirm the 1000-page fixture reports first-build, cached-build, and
  single-page rebuild profile output.
- Run `scripts/release-artifact-smoke.sh` before tagging a release to confirm
  the release workflow still covers Linux x86_64 glibc, Linux aarch64 glibc,
  Linux x86_64 musl, macOS Intel, macOS Apple Silicon, and Windows x86_64
  artifacts.
- After pushing a `v*` tag, confirm the GitHub release contains the six
  expected archives and that each archive contains the `blogx` binary for its
  target.
