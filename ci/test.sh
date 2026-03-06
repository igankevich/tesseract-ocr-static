#!/bin/sh

main() {
    set -ex
    workdir="$(mktemp -d)"
    trap cleanup EXIT
    root="$PWD"
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
    remove_root_dir
    cargo test --workspace --all-features
    create_tar_archive
}

cargo_test_static() {
    remove_root_dir
    case "$(uname -s)" in
    Linux)
        target="$(uname -m)"-unknown-linux-musl
        rustup target add "$target"
        cargo test --target "$target" --workspace --all-features
        create_tar_archive
        ;;
    *)
        env RUSTFLAGS='-C target-feature=+crt-static' \
            cargo test --workspace --all-features
        ;;
    esac
}

create_tar_archive() {
    root_dir="$(find "$root"/target -type d -name __root__)"
    target="$(cat "$root_dir"/../target)"
    version="$(cat "$root_dir"/../version)"
    cd "$root_dir"
    {
        find . -type f -not -name ".*" -print0
        find . -type l -not -name ".*" -print0
    } | env LC_ALL=C sort --zero-terminated >"$workdir"/files
    tar \
        --create \
        --null \
        --files-from="$workdir"/files \
        --numeric-owner \
        --owner=0 \
        --group=0 \
        --file="$root"/root-"$version"-"$target".tar
    zstd -10 --compress "$root"/root-"$version"-"$target".tar
    cd "$root"
}

remove_root_dir() {
    find target -type d -name __root__ | while read -r dir; do rm -rf "$dir"; done
}

cleanup() {
    rm -rf "$workdir"
}

main "$@"
