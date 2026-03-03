#!/bin/sh

main() {
    set -ex
    workdir="$(mktemp -d)"
    trap cleanup EXIT
    cargo_build
    cargo_clippy
    cargo_test
    cargo_test_musl
}

download_tesseract_data() {
    mkdir -p "$workdir"/data
    curl --fail --location -o "$workdir"/data/eng.traineddata \
        https://github.com/tesseract-ocr/tessdata_fast/raw/refs/heads/main/eng.traineddata
    export TESSDATA_PREFIX="$workdir"/data
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

cargo_test_musl() {
    target=x86_64-unknown-linux-musl
    rustup target add "$target"
    cargo test --target "$target" --workspace
}

cleanup() {
    rm -rf "$workdir"
}

main "$@"
