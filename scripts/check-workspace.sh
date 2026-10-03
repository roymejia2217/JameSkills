#!/usr/bin/env bash
# Espejo local de la CI mínima (T006.b): fmt, clippy, suites y check desktop.
# La CI además compila los targets Linux/Windows; este script no empaqueta.
set -euo pipefail

repository_root="$(git rev-parse --show-toplevel)"
cd "$repository_root"

echo "==> cargo fmt --all -- --check"
cargo fmt --all -- --check

echo "==> cargo clippy --workspace --all-targets --features jameskills-desktop/test-support --locked -- -D warnings"
cargo clippy --workspace --all-targets --features jameskills-desktop/test-support --locked -- -D warnings

echo "==> cargo test -p jameskills-core -p jameskills-infra -p jameskills-cli --locked"
cargo test -p jameskills-core -p jameskills-infra -p jameskills-cli --locked

echo "==> cargo check -p jameskills-desktop --locked"
cargo check -p jameskills-desktop --locked

echo "==> git diff --check"
git diff --check

echo "workspace checks: ok"
