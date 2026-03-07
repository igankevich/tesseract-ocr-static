# tesseract-ocr-static-c

This crate bundles Tesseract OCR and Leptonica libraries.
These two libraries are built together with Musl libc and LLVM libcxx and linked statically.
The build should be reproducible since the versions of all libraries are pinned.
Since there are no dependencies one needs to supply images in raw RGB/RGBA/grayscale format to Tesseract.

The build should work with both dynamically and statically linked C libraries,
i.e. `*-gnu` and `*-musl` targets.

Required CLI tools: `cmake`, `make`, `git`, `python3`, `curl`, `tar`, `zstd`.

Required compiler: Clang 20+.


## Environment variables

The following environment variables affect the build process.

| Variable | Default value | Comment |
|----------|---------------|---------|
| `PATH` | | Executable search path |
| `TESSERACT_CC` | `clang` | C compiler |
| `TESSERACT_CXX` | `clang++` | C++ compiler |
| `TESSERACT_AR` | `llvm-ar` | |
| `TESSERACT_RANLIB` | `llvm-ranlib` | |
| `TESSERACT_CFLAGS` | `-O3` | C compiler flags |
| `TESSERACT_CXXFLAGS` | `-O3` | C++ compiler flags |
| `TESSERACT_LDFLAGS` | | Linker flags |
| `TESSERACT_BUILD_FROM_SOURCE` | | If set, Tesseract OCR is built from source; otherwise an attempt is made to download pre-built binary. If the attempt fails, it is built from source. |
| `TESSERACT_PRE_BUILT_ARCHIVE_URL` | | Override URL from which pre-built binary is downloaded. Normally you should have a different URL for each Rust target. |
| `TESSERACT_PRE_BUILT_ARCHIVE_HASH` | | BLAKE2b hash of the pre-built binary archive. Must be set if you've overriden hard-coded archive URLs. Can be computed with `b2sum` CLI tool. |


## High-level interface

The following crate provides ergonomic Rust interface:
[`tesseract-ocr-static`](https://docs.rs/tesseract-ocr-static/latest/tesseract-ocr-static/).
