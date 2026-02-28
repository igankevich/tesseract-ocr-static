use bindgen::callbacks::ParseCallbacks;
use command_error::CommandExt;
use flate2::read::GzDecoder;
use hex_literal::hex;
use sha2::Digest;
use sha2::Sha256;
use std::fs::create_dir_all;
use std::fs::remove_dir_all;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

const CFLAGS: &str = "-O3 -fPIC -fPIE";
const CXXFLAGS: &str = CFLAGS;
const LDFLAGS: &str = "-fPIC -fPIE";

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

static JOB_CLIENT: OnceLock<Option<jobserver::Client>> = OnceLock::new();

fn job_client() -> Option<&'static jobserver::Client> {
    JOB_CLIENT
        .get_or_init(|| unsafe { jobserver::Client::from_env() })
        .as_ref()
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=wrapper.h");
    build_zlib();
    build_libpng();
    build_libjpeg_turbo();
    build_libwebp();
    build_libtiff();
    build_leptonica();
    build_tesseract();
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("root");
    println!("cargo:rustc-link-search={}/lib64", root_dir.display());
    println!("cargo:rustc-link-lib=static=tesseract");
    println!("cargo:rustc-link-lib=static=leptonica");
    println!("cargo:rustc-link-lib=static=tiff");
    println!("cargo:rustc-link-lib=static=webp");
    println!("cargo:rustc-link-lib=static=sharpyuv");
    println!("cargo:rustc-link-lib=static=jpeg");
    println!("cargo:rustc-link-lib=static=png");
    println!("cargo:rustc-link-lib=static=z");
    println!(
        "cargo:rustc-link-search=/gnu/store/lyk51h6jkdapjkbg0wxfxkjj5fq7aii4-gcc-14.3.0-lib/lib"
    );
    println!("cargo:rustc-link-lib=static=stdc++");
    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-DNO_CONSOLE_IO")
        .clang_arg(&format!("-I{}/include", root_dir.display()))
        .clang_arg(
            "-I/gnu/store/wz6d1vxvlijb3837r13r3h0pd4q8609i-clang-20.1.8/lib/clang/20/include",
        )
        .parse_callbacks(Box::new(DoxygenComments))
        .generate()
        .expect("Unable to generate bindings");
    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");
}

#[derive(Debug)]
struct DoxygenComments;

impl ParseCallbacks for DoxygenComments {
    fn process_comment(&self, comment: &str) -> Option<String> {
        //Some(String::new())
        Some(doxygen_rs::transform(comment))
    }
}

fn download_tar_gz(url: &str, sha2: [u8; 32], filename: &str) {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let archive_file = out_dir.join(filename);
    let mut file_writer = HashingWriter::new(BufWriter::new(File::create(&archive_file).unwrap()));
    reqwest::blocking::get(url)
        .unwrap()
        .copy_to(&mut file_writer)
        .unwrap();
    let (hash, mut writer) = file_writer.into_inner();
    writer.flush().unwrap();
    assert_eq!(sha2, hash);
    let mut archive = tar::Archive::new(GzDecoder::new(File::open(&archive_file).unwrap()));
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
        .arg(url)
        .arg(&dir)
        .status_checked()
        .unwrap();
}

fn make_command() -> Command {
    let mut command = Command::new("make");
    if let Some(client) = job_client() {
        client.configure_make(&mut command);
    }
    command
}

fn build_with_cmake(
    archive_dir: &str,
    configure: impl for<'a, 'b> FnOnce(&'a mut Command, &'b Path) -> &'a mut Command,
) {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let build_dir = out_dir.join("build");
    let root_dir = out_dir.join("root");
    let archive_dir = out_dir.join(archive_dir);
    let _ = remove_dir_all(&build_dir);
    create_dir_all(&build_dir).unwrap();
    configure(
        Command::new("cmake")
            .arg(format!("-DCMAKE_INSTALL_PREFIX={}", root_dir.display()))
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .arg(&archive_dir)
            .env("CFLAGS", CFLAGS)
            .env("CXXFLAGS", CXXFLAGS)
            .env("LDFLAGS", LDFLAGS)
            .env("PKG_CONFIG_PATH", root_dir.join("lib64").join("pkgconfig"))
            .current_dir(&build_dir),
        &root_dir,
    )
    .status_checked()
    .unwrap();
    make_command()
        .current_dir(&build_dir)
        .status_checked()
        .unwrap();
    Command::new("make")
        .arg("install")
        .current_dir(&build_dir)
        .status_checked()
        .unwrap();
    let _ = remove_dir_all(&root_dir.join("share").join("man"));
    let _ = remove_dir_all(&root_dir.join("share").join("doc"));
    remove_dir_all(&build_dir).unwrap();
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
    remove_dir_all(&root_dir.join("lib64").join("cmake").join("zlib")).unwrap();
}

fn build_libpng() {
    download_tar_gz(
        &format!("https://downloads.sourceforge.net/project/libpng/libpng16/{LIBPNG_VERSION}/libpng-{LIBPNG_VERSION}.tar.gz"),
        LIBPNG_SHA2,
        &format!("libpng-{LIBPNG_VERSION}.tar.gz"),
    );
    build_with_cmake(&format!("libpng-{LIBPNG_VERSION}"), |command, _root_dir| {
        command.args([
            "-DPNG_SHARED=0",
            "-DPNG_STATIC=1",
            "-DPNG_TESTS=0",
            "-DPNG_TOOLS=0",
        ])
    });
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("root");
    remove_dir_all(&root_dir.join("lib64").join("cmake").join("PNG")).unwrap();
    remove_dir_all(&root_dir.join("lib64").join("libpng")).unwrap();
}

fn build_libjpeg_turbo() {
    let dirname = format!("libjpeg-turbo-{LIBJPEG_TURBO_VERSION}");
    fetch_git(
        &format!("https://github.com/libjpeg-turbo/libjpeg-turbo"),
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
    remove_dir_all(&root_dir.join("lib64").join("cmake").join("libjpeg-turbo")).unwrap();
}

fn build_libtiff() {
    download_tar_gz(
        &format!("https://download.osgeo.org/libtiff/tiff-{LIBTIFF_VERSION}.tar.gz"),
        LIBTIFF_SHA2,
        &format!("tiff-{LIBTIFF_VERSION}.tar.gz"),
    );
    build_with_cmake(&format!("tiff-{LIBTIFF_VERSION}"), |command, _root_dir| {
        command.args([
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
        ])
    });
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("root");
    remove_dir_all(&root_dir.join("lib64").join("cmake").join("tiff")).unwrap();
}

fn build_libwebp() {
    let dirname = format!("libwebp-{LIBWEBP_VERSION}");
    fetch_git(
        &format!("https://github.com/webmproject/libwebp"),
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
    remove_dir_all(&root_dir.join("share").join("WebP").join("cmake")).unwrap();
}

fn build_leptonica() {
    let dirname = format!("leptonica-{LEPTONICA_VERSION}");
    fetch_git(
        &format!("https://github.com/DanBloomberg/leptonica"),
        LEPTONICA_VERSION,
        &dirname,
    );
    build_with_cmake(&dirname, |command, _root_dir| {
        command.env("CPPFLAGS", "-DNO_CONSOLE_IO").args([
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
        &format!("https://github.com/tesseract-ocr/tesseract"),
        TESSERACT_VERSION,
        &dirname,
    );
    // Executable causes troubles with static linking.
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    std::fs::write(
        out_dir.join(&dirname).join("src").join("tesseract.cpp"),
        "int main() { return 0; }",
    )
    .unwrap();
    // Patch UB.
    substitute(
        out_dir.join(&dirname).join("src").join("api").join("capi.cpp"),
        &[
            ("bool bool_is_list_item;", "bool bool_is_list_item = false;"),
            ("bool bool_is_crown;", "bool bool_is_crown = false;"),
        ],
    );
    build_with_cmake(&dirname, |command, _root_dir| {
        command.args([
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

fn substitute(path: impl AsRef<Path>, rules: &[(impl AsRef<str>, impl AsRef<str>)]) {
    let mut text = std::fs::read_to_string(path.as_ref()).unwrap();
    for (value, replacement) in rules {
        text = text.replace(value.as_ref(), replacement.as_ref());
    }
    std::fs::write(path.as_ref(), text.as_bytes()).unwrap();
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
