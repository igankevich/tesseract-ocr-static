#!/bin/sh

main() {
    set -ex
    workdir="$(mktemp -d)"
    trap cleanup EXIT
    download_tesseract_data
    cargo_test
    cargo_test_static
}

download_tesseract_data() {
    mkdir -p "$workdir"/data
    curl --fail --location -o "$workdir"/data/eng.traineddata \
        https://github.com/tesseract-ocr/tessdata_fast/raw/refs/heads/main/eng.traineddata
    export TESSDATA_PREFIX="$workdir"/data
}

cargo_test() {
    cargo test --workspace --all-features
}

cargo_test_static() {
    case "$(uname -s)" in
    Linux)
        target="$(uname -m)"-unknown-linux-musl
        rustup target add "$target"
        cargo test --target "$target" --workspace --all-features
        ;;
    *)
        env RUSTFLAGS='-C target-feature=+crt-static' \
            cargo test --workspace --all-features
        ;;
    esac
}

cleanup() {
    rm -rf "$workdir"
}

main "$@"
