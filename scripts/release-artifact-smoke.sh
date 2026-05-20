#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="$ROOT/.github/workflows/release.yml"

test -f "$WORKFLOW"

for target in \
  x86_64-unknown-linux-gnu \
  aarch64-unknown-linux-gnu \
  x86_64-unknown-linux-musl \
  x86_64-apple-darwin \
  aarch64-apple-darwin \
  x86_64-pc-windows-msvc
do
  grep -Fq "target: $target" "$WORKFLOW"
  grep -Fq "blogx-\${{ matrix.target }}" "$WORKFLOW"
done

grep -Fq "matrix.target == 'x86_64-unknown-linux-musl'" "$WORKFLOW"
grep -Fq "sudo apt-get install -y musl-tools" "$WORKFLOW"
grep -Fq 'cargo build --release --locked --target ${{ matrix.target }}' "$WORKFLOW"
grep -Fq 'tar -C dist -czf "dist-release/blogx-${{ matrix.target }}.tar.gz" blogx' "$WORKFLOW"
grep -Fq 'Compress-Archive -Path dist/blogx.exe -DestinationPath "dist-release/blogx-${{ matrix.target }}.zip"' "$WORKFLOW"
grep -Fq 'actions/upload-artifact@v6' "$WORKFLOW"
grep -Fq 'gh run download "$GITHUB_RUN_ID" --dir release-assets' "$WORKFLOW"
grep -Fq 'gh release create "$GITHUB_REF_NAME" release-assets/*/blogx-* --title "$GITHUB_REF_NAME" --verify-tag' "$WORKFLOW"

echo "release artifact smoke passed"
