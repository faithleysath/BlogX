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
  x86_64-pc-windows-msvc \
  aarch64-pc-windows-msvc
do
  grep -q "target: $target" "$WORKFLOW"
  grep -q "blogx-\${{ matrix.target }}" "$WORKFLOW"
done

grep -q "matrix.target == 'x86_64-unknown-linux-musl'" "$WORKFLOW"
grep -q "sudo apt-get install -y musl-tools" "$WORKFLOW"
grep -q 'cargo build --release --locked --target ${{ matrix.target }}' "$WORKFLOW"
grep -q 'tar -C dist -czf "blogx-${{ matrix.target }}.tar.gz" blogx' "$WORKFLOW"
grep -q 'Compress-Archive -Path dist/blogx.exe -DestinationPath "blogx-${{ matrix.target }}.zip"' "$WORKFLOW"
grep -q 'softprops/action-gh-release@v2' "$WORKFLOW"
grep -q 'blogx-${{ matrix.target }}.tar.gz' "$WORKFLOW"
grep -q 'blogx-${{ matrix.target }}.zip' "$WORKFLOW"

echo "release artifact smoke passed"
