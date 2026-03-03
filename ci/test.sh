#!/bin/sh

main() {
    set -ex
    workdir="$(mktemp -d)"
    trap cleanup EXIT
    cargo_build
    cargo_clippy
    cargo_test
    #cargo_test_musl
}

cargo_build() {
    cargo build --workspace --all-features
}

cargo_clippy() {
    cargo clippy --workspace --quiet --all-features --all-targets -- --deny warnings
}

cargo_test() {
    cargo test --workspace
}

#cargo_test_musl() {
#    cargo test --target --workspace
#}

cleanup() {
    rm -rf "$workdir"
}

main "$@"
