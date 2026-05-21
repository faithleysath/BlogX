#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WORKFLOW="$ROOT/.github/workflows/release.yml"

test -f "$WORKFLOW"

for target in \
  x86_64-unknown-linux-gnu \
  aarch64-unknown-linux-gnu \
  x86_64-apple-darwin \
  aarch64-apple-darwin
do
  grep -Fq "target: $target" "$WORKFLOW"
  grep -Fq "blogx-\${{ matrix.target }}" "$WORKFLOW"
done

! grep -Fq "x86_64-unknown-linux-musl" "$WORKFLOW"
! grep -Fq "x86_64-pc-windows-msvc" "$WORKFLOW"
grep -Fq 'cargo build --release --locked --target ${{ matrix.target }}' "$WORKFLOW"
grep -Fq 'tar -C dist -czf "dist-release/blogx-${{ matrix.target }}.tar.gz" blogx' "$WORKFLOW"
grep -Fq 'actions/upload-artifact@v6' "$WORKFLOW"
grep -Fq 'gh run download "$GITHUB_RUN_ID" --dir release-assets' "$WORKFLOW"
grep -Fq 'gh release create "$GITHUB_REF_NAME" release-assets/*/blogx-* --title "$GITHUB_REF_NAME" --verify-tag' "$WORKFLOW"
grep -Fq 'name: Publish npm' "$WORKFLOW"
grep -Fq 'working-directory: npm' "$WORKFLOW"
grep -Fq 'npm pack --dry-run' "$WORKFLOW"
grep -Fq 'npm publish --access public --registry=https://registry.npmjs.org' "$WORKFLOW"

echo "release artifact smoke passed"
