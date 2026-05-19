#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

SITE="$TMP_DIR/site"
"$ROOT/target/debug/blogx" init "$SITE" >/dev/null

cat >"$SITE/content/index.md" <<'MARKDOWN'
---
title: Home
---

# Home

[About](about.md)

Inline math $a^2 + b^2 = c^2$.

$$
\int_0^1 x^2 dx
$$

```rust
fn main() {
    println!("hello");
}
```

![Logo](img/logo.svg)
MARKDOWN

mkdir -p "$SITE/content/img"
cat >"$SITE/content/img/logo.svg" <<'SVG'
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 40" role="img" aria-label="Logo">
  <rect width="120" height="40" fill="#176b87"/>
  <text x="12" y="26" fill="white" font-size="18">BlogX</text>
</svg>
SVG

run_checks() {
  local site="$1"
  local fingerprint="$2"
  (cd "$site" && "$ROOT/target/debug/blogx" build --profile >/tmp/blogx-no-js-build.out)

  local html="$site/public/index.html"
  test -f "$html"
  grep -q '<a href="about.html">About</a>' "$html"
  grep -q 'class="katex' "$html"
  grep -q 'class="line-number"' "$html"
  grep -q 'alt="Logo"' "$html"
  grep -q '<script src="/assets/theme/js/enhance' "$html"
  grep -q '<link rel="stylesheet" href="/assets/theme/css/main' "$html"
  grep -q '<link rel="stylesheet" href="/assets/theme/css/katex' "$html"
  grep -q '<link rel="stylesheet" href="/assets/theme/css/syntax' "$html"
  test -f "$site/public/img/logo.svg"

  if [ "$fingerprint" = true ]; then
    local katex_css
    katex_css="$(find "$site/public/assets/theme/css" -maxdepth 1 -name 'katex.*.css' -print -quit)"
    test -n "$katex_css"
    grep -q 'url(fonts/KaTeX_Main-Regular\.' "$katex_css"
    ! grep -q 'url(fonts/KaTeX_Main-Regular.woff2)' "$katex_css"
    local font_url
    font_url="$(grep -o 'fonts/KaTeX_Main-Regular[^)]*' "$katex_css" | head -n1)"
    test -f "$site/public/assets/theme/css/$font_url"
  fi
}

run_checks "$SITE" false

sed -i 's/fingerprint = false/fingerprint = true/' "$SITE/blogx.toml"
run_checks "$SITE" true

echo "no-js smoke passed"
