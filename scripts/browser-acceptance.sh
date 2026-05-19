#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

SITE="$TMP_DIR/site"
"$ROOT/target/debug/blogx" init "$SITE" >/dev/null

cat >"$SITE/content/index.md" <<'MARKDOWN'
---
title: Browser Acceptance
---

# Browser Acceptance

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
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 240 80" role="img" aria-label="Logo">
  <rect width="240" height="80" fill="#176b87"/>
  <text x="24" y="52" fill="white" font-size="36">BlogX</text>
</svg>
SVG

(cd "$SITE" && "$ROOT/target/debug/blogx" build >/dev/null)

HTML="$SITE/public/index.html"
grep -q 'class="katex' "$HTML"
grep -q 'class="line-number"' "$HTML"
grep -q '<a href="about.html">About</a>' "$HTML"
grep -q 'alt="Logo"' "$HTML"

PROFILE="$TMP_DIR/firefox-profile"
mkdir -p "$PROFILE"
cat >"$PROFILE/user.js" <<'JS'
user_pref("javascript.enabled", false);
user_pref("dom.disable_open_during_load", true);
JS

firefox --headless --profile "$PROFILE" --window-size 1280,900 --screenshot "$TMP_DIR/desktop.png" "file://$HTML" >/dev/null 2>&1
firefox --headless --profile "$PROFILE" --window-size 390,844 --screenshot "$TMP_DIR/mobile.png" "file://$HTML" >/dev/null 2>&1

test -s "$TMP_DIR/desktop.png"
test -s "$TMP_DIR/mobile.png"

desktop_colors="$(identify -format '%k' "$TMP_DIR/desktop.png")"
mobile_colors="$(identify -format '%k' "$TMP_DIR/mobile.png")"
if [ "$desktop_colors" -lt 8 ] || [ "$mobile_colors" -lt 8 ]; then
  echo "browser screenshots look blank" >&2
  exit 1
fi

echo "browser acceptance passed"
