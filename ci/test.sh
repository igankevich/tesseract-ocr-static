#!/bin/sh

main() {
    set -ex
    workdir="$(mktemp -d)"
    trap cleanup EXIT
    cargo_build_vv
    cargo_clippy
    cargo_test
}

cargo_build_vv() {
    cargo build -vv --workspace --all-features
}

cargo_clippy() {
    cargo clippy --workspace --quiet --all-features --all-targets -- --deny warnings
}

cargo_test() {
    cargo test --workspace
}

cleanup() {
    rm -rf "$workdir"
}

main "$@"
