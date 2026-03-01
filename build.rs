#![allow(clippy::unwrap_used)]

use std::ffi::OsStr;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use bindgen::callbacks::ParseCallbacks;
use command_error::CommandExt;
use flate2::read::GzDecoder;
use hex_literal::hex;
use sha2::Digest;
use sha2::Sha256;

const CFLAGS: &str = "-O3 -fPIC -fPIE -D_GNU_SOURCE -I/gnu/store/wmgiqgp0mzyihvy24b70rm9x2qrmnvxy-linux-libre-headers-6.12.17/include";
const LDFLAGS: &str = "-fPIC -fPIE";

const MUSL_VERSION: &str = "1.2.5";
const MUSL_SHA2: [u8; 32] =
    hex!("83ff394502d1c334b040ea9bc66ec48bba453585e25b05f4bde3741d8245d883");
const ZLIB_VERSION: &str = "1.3.2";
const ZLIB_SHA2: [u8; 32] =
    hex!("bb329a0a2cd0274d05519d61c667c062e06990d72e125ee2dfa8de64f0119d16");
const LIBPNG_VERSION: &str = "1.6.55";
const LIBPNG_SHA2: [u8; 32] =
    hex!("4b0abab6d219e95690ebe4db7fc9aa95f4006c83baaa022373c0c8442271283d");
const LIBTIFF_VERSION: &str = "4.7.1";
const LIBTIFF_SHA2: [u8; 32] =
    hex!("f698d94f3103da8ca7438d84e0344e453fe0ba3b7486e04c5bf7a9a3fabe9b69");
const LIBJPEG_TURBO_VERSION: &str = "3.1.3";
const LIBWEBP_VERSION: &str = "1.6.0";
const LEPTONICA_VERSION: &str = "1.87.0";
const TESSERACT_VERSION: &str = "5.5.2";
const LIBCXX_VERSION: &str = "22.1.0";

static JOB_CLIENT: OnceLock<Option<jobserver::Client>> = OnceLock::new();

fn job_client() -> Option<&'static jobserver::Client> {
    JOB_CLIENT
        .get_or_init(|| unsafe { jobserver::Client::from_env() })
        .as_ref()
}

fn root_dir() -> PathBuf {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    out_dir.join("root")
}

fn c_flags() -> String {
    if is_musl_target() {
        format!(
            "{CFLAGS} --sysroot {} -isystem {}",
            root_dir().display(),
            root_dir().join("include").display()
        )
    } else {
        format!("{CFLAGS} -I{}", root_dir().join("include").display())
    }
}

fn cxx_flags() -> String {
    format!(
        "{} -nostdinc++ -fno-exceptions -I{}",
        c_flags(),
        root_dir().join("include").join("c++").join("v1").display()
    )
}

fn ld_flags() -> String {
    if is_musl_target() {
        format!(
            "{LDFLAGS} --sysroot {} -Wl,-L{} -nostdlib -Wl,-lc",
            root_dir().display(),
            root_dir().join("lib").display(),
        )
    } else {
        format!("{LDFLAGS} -Wl,-L{}", root_dir().join("lib").display(),)
    }
}

fn is_musl_target() -> bool {
    std::env::var_os("CARGO_CFG_TARGET_ENV").as_deref() == Some(OsStr::new("musl"))
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=wrapper.h");
    println!("cargo::rerun-if-env-changed=CC");
    println!("cargo::rerun-if-env-changed=CXX");
    println!("cargo::rerun-if-env-changed=PATH");
    if is_musl_target() {
        build_musl();
    }
    build_zlib();
    build_libpng();
    build_libjpeg_turbo();
    build_libwebp();
    build_libcxx();
    build_libtiff();
    build_leptonica();
    build_tesseract();
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("root");
    println!("cargo:rustc-link-search={}/lib", root_dir.display());
    println!("cargo:rustc-link-lib=static=tesseract");
    println!("cargo:rustc-link-lib=static=leptonica");
    println!("cargo:rustc-link-lib=static=tiff");
    println!("cargo:rustc-link-lib=static=webp");
    println!("cargo:rustc-link-lib=static=sharpyuv");
    println!("cargo:rustc-link-lib=static=jpeg");
    println!("cargo:rustc-link-lib=static=png");
    println!("cargo:rustc-link-lib=static=z");
    println!("cargo:rustc-link-lib=static=c++");
    println!("cargo:rustc-link-lib=static=c++abi");
    //println!("cargo:rustc-link-lib=static=c");
    //println!("cargo:rustc-link-arg=-Wl,-nostdlib");
    //println!("cargo:rustc-link-arg=-Wl,-nolibc");
    let builder = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-DNO_CONSOLE_IO")
        .clang_arg("-std=c23")
        .clang_arg(format!("-I{}/include", root_dir.display()))
        .parse_callbacks(Box::new(IgnoreComments));
    let builder = if let Some(path) = std::env::var_os("LIBCLANG_INCLUDE") {
        builder.clang_arg(format!("-I{}", path.display()))
    } else {
        builder
    };
    let bindings = builder.generate().expect("Unable to generate bindings");
    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

#[derive(Debug)]
struct IgnoreComments;

impl ParseCallbacks for IgnoreComments {
    fn process_comment(&self, _comment: &str) -> Option<String> {
        Some(String::new())
    }
}

fn download_tar_gz(url: &str, sha2: [u8; 32], filename: &str) {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let archive_file = out_dir.join(filename);
    let mut file_writer =
        HashingWriter::new(BufWriter::new(fs::File::create(&archive_file).unwrap()));
    reqwest::blocking::get(url)
        .unwrap()
        .copy_to(&mut file_writer)
        .unwrap();
    let (hash, mut writer) = file_writer.into_inner();
    writer.flush().unwrap();
    assert_eq!(sha2, hash);
    let mut archive = tar::Archive::new(GzDecoder::new(fs::File::open(&archive_file).unwrap()));
    archive.unpack(&out_dir).unwrap();
}

fn fetch_git(url: &str, tag: &str, dirname: &str) {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let dir = out_dir.join(dirname);
    if dir.exists() {
        return;
    }
    Command::new("git")
        .arg("clone")
        .arg(format!("--revision={tag}"))
        .arg("--depth=1")
        .arg(url)
        .arg(&dir)
        .status_checked()
        .unwrap();
}

fn hermetic_command(command: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(command);
    command.env_clear();
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    command
}

fn make_command() -> Command {
    let mut command = hermetic_command("make");
    if let Some(client) = job_client() {
        client.configure_make(&mut command);
    }
    command
}

fn configure_with_cmake(
    archive_dir: impl AsRef<Path>,
    configure: impl for<'a, 'b> FnOnce(&'a mut Command, &'b Path) -> &'a mut Command,
) {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let archive_dir = out_dir.join(archive_dir.as_ref());
    let build_dir = archive_dir.join("__build__");
    let root_dir = out_dir.join("root");
    let _ = fs::remove_dir_all(&build_dir);
    fs::create_dir_all(&build_dir).unwrap();
    configure(
        hermetic_command("cmake")
            .arg(format!("-DCMAKE_INSTALL_PREFIX={}", root_dir.display()))
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .arg("-DCMAKE_INSTALL_LIBDIR=lib")
            .arg(&archive_dir)
            .env(
                "CC",
                std::env::var_os("CC")
                    .as_deref()
                    .unwrap_or(OsStr::new("clang")),
            )
            .env(
                "CXX",
                std::env::var_os("CXX")
                    .as_deref()
                    .unwrap_or(OsStr::new("clang++")),
            )
            .env("CFLAGS", c_flags())
            .env("CXXFLAGS", cxx_flags())
            .env("LDFLAGS", ld_flags())
            .env("PKG_CONFIG_PATH", root_dir.join("lib").join("pkgconfig"))
            .current_dir(&build_dir),
        &root_dir,
    )
    .status_checked()
    .unwrap();
}

fn make_with_cmake(build_dir: impl AsRef<Path>) {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("root");
    make_command()
        .arg("VERBOSE=1")
        .current_dir(&build_dir)
        .status_checked()
        .unwrap();
    hermetic_command("make")
        .arg("install")
        .current_dir(&build_dir)
        .status_checked()
        .unwrap();
    let _ = fs::remove_dir_all(root_dir.join("share").join("man"));
    let _ = fs::remove_dir_all(root_dir.join("share").join("doc"));
    fs::remove_dir_all(&build_dir).unwrap();
}

fn build_with_cmake(
    archive_dir: impl AsRef<Path>,
    configure: impl for<'a, 'b> FnOnce(&'a mut Command, &'b Path) -> &'a mut Command,
) {
    configure_with_cmake(archive_dir.as_ref(), configure);
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let build_dir = out_dir.join(archive_dir).join("__build__");
    make_with_cmake(build_dir);
}

fn build_musl() {
    download_tar_gz(
        &format!("https://git.musl-libc.org/cgit/musl/snapshot/musl-{MUSL_VERSION}.tar.gz"),
        MUSL_SHA2,
        &format!("musl-{MUSL_VERSION}.tar.gz"),
    );
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let archive_dir = out_dir.join(format!("musl-{MUSL_VERSION}"));
    let build_dir = archive_dir.join("__build__");
    let root_dir = out_dir.join("root");
    let _ = fs::remove_dir_all(&build_dir);
    fs::create_dir_all(&build_dir).unwrap();
    hermetic_command(archive_dir.join("configure"))
        .current_dir(&build_dir)
        .arg(format!("--prefix={}", root_dir.display()))
        .arg("--enable-wrapper=clang")
        .arg("--disable-shared=clang")
        .env("AR", "llvm-ar")
        .env("RANLIB", "llvm-ranlib")
        .env("CC", "clang")
        .env("CPPFLAGS", "-nostdinc")
        .env("CFLAGS", "-O3 -fPIC -fPIE")
        .env("LDFLAGS", "-fPIC -fPIE")
        .status_checked()
        .unwrap();
    make_with_cmake(&build_dir);
}

fn build_zlib() {
    download_tar_gz(
        &format!("https://zlib.net/zlib-{ZLIB_VERSION}.tar.gz"),
        ZLIB_SHA2,
        &format!("zlib-{ZLIB_VERSION}.tar.gz"),
    );
    build_with_cmake(&format!("zlib-{ZLIB_VERSION}"), |command, _root_dir| {
        command.args([
            "-DZLIB_BUILD_TESTING=0",
            "-DZLIB_BUILD_SHARED=0",
            "-DZLIB_BUILD_STATIC=1",
        ])
    });
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("root");
    fs::remove_dir_all(root_dir.join("lib").join("cmake").join("zlib")).unwrap();
}

fn build_libpng() {
    download_tar_gz(
        &format!(
            "https://downloads.sourceforge.net/project/libpng/libpng16/{LIBPNG_VERSION}/libpng-{LIBPNG_VERSION}.tar.gz"
        ),
        LIBPNG_SHA2,
        &format!("libpng-{LIBPNG_VERSION}.tar.gz"),
    );
    build_with_cmake(&format!("libpng-{LIBPNG_VERSION}"), |command, root_dir| {
        command
            .arg(format!("-DZLIB_ROOT={}", root_dir.display()))
            .args([
                "-DPNG_SHARED=0",
                "-DPNG_STATIC=1",
                "-DPNG_TESTS=0",
                "-DPNG_TOOLS=0",
            ])
    });
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("root");
    fs::remove_dir_all(root_dir.join("lib").join("cmake").join("PNG")).unwrap();
    fs::remove_dir_all(root_dir.join("lib").join("libpng")).unwrap();
}

fn build_libjpeg_turbo() {
    let dirname = format!("libjpeg-turbo-{LIBJPEG_TURBO_VERSION}");
    fetch_git(
        "https://github.com/libjpeg-turbo/libjpeg-turbo",
        LIBJPEG_TURBO_VERSION,
        &dirname,
    );
    build_with_cmake(&dirname, |command, _root_dir| {
        command.args([
            "-DENABLE_STATIC=1",
            "-DENABLE_SHARED=0",
            "-DWITH_TOOLS=0",
            "-DWITH_TESTS=0",
            "-DWITH_TURBOJPEG=0",
        ])
    });
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("root");
    fs::remove_dir_all(root_dir.join("lib").join("cmake").join("libjpeg-turbo")).unwrap();
}

fn build_libtiff() {
    download_tar_gz(
        &format!("https://download.osgeo.org/libtiff/tiff-{LIBTIFF_VERSION}.tar.gz"),
        LIBTIFF_SHA2,
        &format!("tiff-{LIBTIFF_VERSION}.tar.gz"),
    );
    build_with_cmake(&format!("tiff-{LIBTIFF_VERSION}"), |command, root_dir| {
        command
            .arg(format!("-DZLIB_ROOT={}", root_dir.display()))
            .args([
                "-DBUILD_SHARED_LIBS=0",
                "-Dtiff-static=1",
                "-Dtiff-tools=0",
                "-Dtiff-tests=0",
                "-Dtiff-contrib=0",
                "-Dtiff-docs=0",
                "-Dtiff-install=1",
                "-Dwebp=1",
                "-Dzlib=1",
                "-Djpeg=1",
                "-Dzlib=1",
                "-Dzstd=0",
                "-Dlzma=0",
            ])
    });
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("root");
    fs::remove_dir_all(root_dir.join("lib").join("cmake").join("tiff")).unwrap();
}

fn build_libwebp() {
    let dirname = format!("libwebp-{LIBWEBP_VERSION}");
    fetch_git(
        "https://github.com/webmproject/libwebp",
        &format!("v{LIBWEBP_VERSION}"),
        &dirname,
    );
    build_with_cmake(&dirname, |command, _root_dir| {
        command.args([
            "-DBUILD_SHARED_LIBS=0",
            "-DWEBP_LINK_STATIC=1",
            "-DWEBP_BUILD_ANIM_UTILS=0",
            "-DWEBP_BUILD_CWEBP=0",
            "-DWEBP_BUILD_DWEBP=0",
            "-DWEBP_BUILD_GIF2WEBP=0",
            "-DWEBP_BUILD_IMG2WEBP=0",
            "-DWEBP_BUILD_VWEBP=0",
            "-DWEBP_BUILD_WEBPINFO=0",
            "-DWEBP_BUILD_LIBWEBPMUX=1",
            "-DWEBP_BUILD_WEBPMUX=0",
            "-DWEBP_BUILD_EXTRAS=0",
            "-DWEBP_BUILD_WEBP_JS=0",
            "-DWEBP_BUILD_FUZZTEST=0",
            "-DWEBP_USE_THREAD=0",
        ])
    });
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("root");
    fs::remove_dir_all(root_dir.join("share").join("WebP").join("cmake")).unwrap();
}

fn build_leptonica() {
    let dirname = format!("leptonica-{LEPTONICA_VERSION}");
    fetch_git(
        "https://github.com/DanBloomberg/leptonica",
        LEPTONICA_VERSION,
        &dirname,
    );
    build_with_cmake(&dirname, |command, root_dir| {
        let more_c_flags = format!("{} -DNO_CONSOLE_IO", c_flags());
        command
            .arg(format!("-DZLIB_ROOT={}", root_dir.display()))
            .env("CFLAGS", more_c_flags)
            .args([
                "-DBUILD_SHARED_LIBS=0",
                "-DSTRICT_CONF=1",
                "-DENABLE_ZLIB=1",
                "-DENABLE_PNG=1",
                "-DENABLE_GIF=0",
                "-DENABLE_JPEG=1",
                "-DENABLE_TIFF=1",
                "-DENABLE_WEBP=1",
                "-DENABLE_OPENJPEG=0",
            ])
    });
}

fn build_tesseract() {
    let dirname = format!("tesseract-{TESSERACT_VERSION}");
    fetch_git(
        "https://github.com/tesseract-ocr/tesseract",
        TESSERACT_VERSION,
        &dirname,
    );
    // Executable causes troubles with static linking.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    fs::write(
        out_dir.join(&dirname).join("src").join("tesseract.cpp"),
        "int main() { return 0; }",
    )
    .unwrap();
    // Patch UB.
    substitute(
        out_dir
            .join(&dirname)
            .join("src")
            .join("api")
            .join("capi.cpp"),
        &[
            ("bool bool_is_list_item;", "bool bool_is_list_item = false;"),
            ("bool bool_is_crown;", "bool bool_is_crown = false;"),
        ],
    );
    build_with_cmake(&dirname, |command, _root_dir| {
        let more_cxx_flags = format!("{} -DNO_CONSOLE_IO", cxx_flags());
        command.env("CXXFLAGS", more_cxx_flags).args([
            "-DBUILD_SHARED_LIBS=0",
            "-DGRAPHICS_DISABLED=1",
            "-DDISABLED_LEGACY_ENGINE=0",
            "-DBUILD_TRAINING_TOOLS=0",
            "-DBUILD_TESTS=0",
            "-DDISABLE_ARCHIVE=1",
            "-DDISABLE_CURL=1",
        ])
    });
}

fn build_libcxx() {
    let dirname = format!("libcxx-{LIBCXX_VERSION}");
    fetch_git(
        "https://github.com/llvm/llvm-project",
        &format!("llvmorg-{LIBCXX_VERSION}"),
        &dirname,
    );
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let _ = fs::remove_dir_all(out_dir.join("root").join("include").join("c++"));
    build_with_cmake(
        &Path::new(&dirname).join("libunwind"),
        |command, _root_dir| {
            command.args([
                "-DLIBUNWIND_ENABLE_SHARED=0",
                "-DLIBUNWIND_ENABLE_STATIC=1",
                "-DLIBUNWIND_USE_COMPILER_RT=1",
                "-DLIBUNWIND_INCLUDE_DOCS=0",
                "-DLIBUNWIND_INCLUDE_TESTS=0",
            ])
        },
    );
    configure_with_cmake(&Path::new(&dirname).join("libcxx"), |command, root_dir| {
        let cxx_flags = format!(
            "{} -nostdinc++ {} -I{}",
            if is_musl_target() { "-nostdinc" } else { "" },
            cxx_flags(),
            root_dir.join("include").join("c++").join("v1").display(),
        );
        eprintln!("Override libcxx CXXFLAGS = {cxx_flags:?}");
        if is_musl_target() {
            command.arg("-DLIBCXX_HAS_MUSL_LIBC=1");
        }
        command.env("CXXFLAGS", &cxx_flags).args([
            "-DLIBCXX_ENABLE_EXCEPTIONS=0",
            "-DLIBCXX_ENABLE_SHARED=0",
            "-DLIBCXX_ENABLE_STATIC=1",
            "-DLIBCXX_INCLUDE_TESTS=0",
            "-DLIBCXX_INCLUDE_BENCHMARKS=0",
            "-DLIBCXX_INCLUDE_DOCS=0",
            "-DLIBCXX_USE_COMPILER_RT=1",
            "-DLIBCXXABI_USE_LLVM_UNWINDER=0",
            "-DLIBCXX_ENABLE_STATIC_ABI_LIBRARY=1",
            "-DPython3_EXECUTABLE=python3",
        ])
    });
    build_with_cmake(
        &Path::new(&dirname).join("libcxxabi"),
        |command, _root_dir| {
            let cxx_flags = format!(
                "{} -nostdinc++ -I{} -I{} {}",
                if is_musl_target() { "-nostdinc" } else { "" },
                out_dir
                    .join(&dirname)
                    .join("libcxx")
                    .join("__build__")
                    .join("include")
                    .join("c++")
                    .join("v1")
                    .display(),
                out_dir
                    .join(&dirname)
                    .join("libcxx")
                    .join("include")
                    .display(),
                cxx_flags(),
            );
            eprintln!("Override CXXFLAGS = {cxx_flags:?}");
            command.env("CXXFLAGS", &cxx_flags).args([
                "-DLIBCXXABI_ENABLE_EXCEPTIONS=0",
                "-DLIBCXXABI_SILENT_TERMINATE=1",
                "-DLIBCXXABI_USE_LLVM_UNWINDER=0",
                "-DLIBCXXABI_ENABLE_STATIC_UNWINDER=1",
                "-DLIBCXXABI_USE_COMPILER_RT=1",
                "-DLIBCXXABI_ENABLE_SHARED=0",
                "-DLIBCXXABI_ENABLE_STATIC=1",
                "-DLIBCXXABI_INCLUDE_TESTS=0",
            ])
        },
    );
    build_with_cmake(&Path::new(&dirname).join("libcxx"), |command, root_dir| {
        let cxx_flags = format!(
            "{} -nostdinc++ {} -I{}",
            if is_musl_target() { "-nostdinc" } else { "" },
            cxx_flags(),
            root_dir.join("include").join("c++").join("v1").display(),
        );
        eprintln!("Override libcxx CXXFLAGS = {cxx_flags:?}");
        if is_musl_target() {
            command.arg("-DLIBCXX_HAS_MUSL_LIBC=1");
        }
        command
            .env("CXXFLAGS", &cxx_flags)
            .arg(format!("-DCMAKE_CXX_FLAGS_RELEASE={cxx_flags}"))
            .args([
                "-DLIBCXX_ENABLE_EXCEPTIONS=0",
                "-DLIBCXX_ENABLE_SHARED=0",
                "-DLIBCXX_ENABLE_STATIC=1",
                "-DLIBCXX_INCLUDE_TESTS=0",
                "-DLIBCXX_INCLUDE_BENCHMARKS=0",
                "-DLIBCXX_INCLUDE_DOCS=0",
                "-DLIBCXX_USE_COMPILER_RT=1",
                "-DLIBCXXABI_USE_LLVM_UNWINDER=0",
                "-DLIBCXX_ENABLE_STATIC_ABI_LIBRARY=1",
                "-DPython3_EXECUTABLE=python3",
            ])
    });
}

fn substitute(path: impl AsRef<Path>, rules: &[(impl AsRef<str>, impl AsRef<str>)]) {
    let mut text = fs::read_to_string(path.as_ref()).unwrap();
    for (value, replacement) in rules {
        text = text.replace(value.as_ref(), replacement.as_ref());
    }
    fs::write(path.as_ref(), text.as_bytes()).unwrap();
}

struct HashingWriter<W: Write> {
    hasher: Sha256,
    writer: W,
}

impl<W: Write> HashingWriter<W> {
    fn new(writer: W) -> Self {
        let hasher = Sha256::new();
        Self { writer, hasher }
    }

    fn into_inner(self) -> ([u8; 32], W) {
        let hash = self.hasher.finalize().into();
        (hash, self.writer)
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.hasher.update(buf);
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}
