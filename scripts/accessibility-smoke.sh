#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

SITE="$TMP_DIR/site"
"$ROOT/target/debug/blogx" init "$SITE" >/dev/null

cat >"$SITE/content/index.md" <<'MARKDOWN'
---
title: Accessibility
---

# Accessibility

[About](about.md)

![Logo](img/logo.svg)

```rust
fn main() {}
```

Inline math $x + y$.
MARKDOWN

mkdir -p "$SITE/content/img"
cat >"$SITE/content/img/logo.svg" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 40" role="img" aria-label="Logo">
  <rect width="120" height="40" fill="#176b87"/>
  <text x="12" y="26" fill="white" font-size="18">BlogX</text>
</svg>
SVG

(cd "$SITE" && "$ROOT/target/debug/blogx" build >/tmp/blogx-accessibility-build.out)

HTML="$SITE/public/index.html"
CSS="$SITE/public/assets/theme/css/main.css"

test -f "$HTML"
test -f "$CSS"

grep -q '<html lang="zh">' "$HTML"
grep -q '<a class="skip-link" href="#content">Skip to content</a>' "$HTML"
grep -q '<main id="content" class="site-main">' "$HTML"
grep -q '<article class="page">' "$HTML"
grep -q '<nav class="site-nav" aria-label="Primary">' "$HTML"
grep -q '<footer class="site-footer">' "$HTML"
grep -q 'alt="Logo"' "$HTML"
grep -q 'class="line-number" aria-hidden="true"' "$HTML"
grep -q 'class="katex' "$HTML"
grep -q '<script src="/assets/theme/js/enhance.js" defer></script>' "$HTML"

if grep -Eq '<img[^>]*alt=""' "$HTML"; then
  echo "generated HTML contains an empty image alt" >&2
  exit 1
fi

grep -q ':focus-visible' "$CSS"
grep -q 'prefers-reduced-motion' "$CSS"
grep -q 'max-width: 100%' "$CSS"
grep -q 'height: auto' "$CSS"

echo "accessibility smoke passed"
