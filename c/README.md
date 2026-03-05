# tesseract-ocr-static-c

This crate bundles Tesseract OCR and Leptonica libraries.
These two libraries are built together with Musl libc and LLVM libcxx and linked statically.
The build should be reproducible since the versions of all libraries are pinned.
Since there are no dependencies one needs to supply images in raw RGB/RGBA/grayscale format to Tesseract.

The build should work with both dynamically and statically linked C libraries,
i.e. `*-gnu` and `*-musl` targets.

Required CLI tools: `cmake`, `make`, `git`, `python3`.

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
