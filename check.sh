#!/usr/bin/env bash

set -euo pipefail

# checks that are meant to be in-sync with CI

echo 'Running `cargo fmt --all -- --check`..'
cargo fmt --all -- --check

echo 'Running `cargo clippy -- -D warnings`..'
cargo clippy -- -D warnings

echo 'Running `cargo test`..'
cargo test
