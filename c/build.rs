#![allow(clippy::unwrap_used)]

use std::env::var_os;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::ExitStatus;
use std::sync::LazyLock;
use std::sync::OnceLock;

use bindgen::callbacks::ParseCallbacks;

const MUSL_VERSION: &str = "1.2.5";
const LIBCXX_VERSION: &str = "22.1.0";
const LEPTONICA_VERSION: &str = "1.87.0";
const TESSERACT_VERSION: &str = "5.5.2";

macro_rules! pre_built_archive {
    ($target: literal, $hash: literal) => {
        (
            $target,
            concat!(
                "https://github.com/igankevich/tesseract-ocr-static/releases/download/",
                env!("CARGO_PKG_VERSION"),
                "/root-",
                env!("CARGO_PKG_VERSION"),
                "-",
                $target,
                ".tar.zst"
            ),
            $hash,
        )
    };
}

const PRE_BUILT_ARCHIVES: &[(&str, &str, &str)] = &[
    pre_built_archive!(
        "x86_64-unknown-linux-gnu",
        "de1660df7d25ad5c0ed95ee01b1e5ec7b48c01d0e153aa723b1b797665556ab7014038f267ef820a4a8f1f26234368abe2ec01f4b913d0484553635cc6556b14"
    ),
    pre_built_archive!(
        "x86_64-unknown-linux-musl",
        "4e5f1cc01395f3af23636f39c4a20fb6fdbf67ed24957eaea7c08453266bb60c814a956a9124618a6e0cef5303f4885c3527f823753bfca5622dcae70fd2da6f"
    ),
    pre_built_archive!(
        "aarch64-apple-darwin",
        "51312821f580c7227ee2815d897c7579a6506efdcf3203691d29b36ba5589e517a8c342cd81a6c2e30eb810f21c2dffa7c0053e513272a96902a30004b459804"
    ),
];

static TESSERACT_CC: LazyLock<OsString> = LazyLock::new(|| tess_var("CC", "clang"));
static TESSERACT_CXX: LazyLock<OsString> = LazyLock::new(|| tess_var("CXX", "clang++"));
static TESSERACT_AR: LazyLock<OsString> = LazyLock::new(|| tess_var("AR", "llvm-ar"));
static TESSERACT_RANLIB: LazyLock<OsString> = LazyLock::new(|| tess_var("RANLIB", "llvm-ranlib"));
static TESSERACT_CFLAGS: LazyLock<OsString> = LazyLock::new(|| tess_var("CFLAGS", "-O3"));
static TESSERACT_CXXFLAGS: LazyLock<OsString> = LazyLock::new(|| tess_var("CXXFLAGS", "-O3"));
static TESSERACT_LDFLAGS: LazyLock<OsString> = LazyLock::new(|| tess_var("LDFLAGS", ""));

const COMMON_CFLAGS: &str = "-fPIC -fPIE -D_GNU_SOURCE";
const COMMON_LDFLAGS: &str = "-fPIC -fPIE";

static CFLAGS: LazyLock<OsString> = LazyLock::new(|| {
    let mut flags = OsString::new();
    flags.push(&*TESSERACT_CFLAGS);
    flags.push(" ");
    flags.push(COMMON_CFLAGS);
    if is_static_build() {
        flags.push(" -isystem ");
        flags.push(root_dir().join("include"));
    } else {
        flags.push(" -I");
        flags.push(root_dir().join("include"));
    }
    flags
});

static CXXFLAGS: LazyLock<OsString> = LazyLock::new(|| {
    let mut flags = OsString::new();
    flags.push(&*TESSERACT_CXXFLAGS);
    flags.push(" ");
    flags.push(COMMON_CFLAGS);
    flags.push(" -nostdinc++ -fno-exceptions -I");
    flags.push(root_dir().join("include").join("c++").join("v1"));
    if is_static_build() {
        flags.push(" -isystem ");
        flags.push(root_dir().join("include"));
    } else {
        flags.push(" -I");
        flags.push(root_dir().join("include"));
    }
    flags
});

static LDFLAGS: LazyLock<OsString> = LazyLock::new(|| {
    let mut flags = OsString::new();
    flags.push(&*TESSERACT_LDFLAGS);
    flags.push(" ");
    flags.push(COMMON_LDFLAGS);
    flags.push(" -Wl,-L");
    flags.push(root_dir().join("lib"));
    if is_static_build() {
        flags.push(" -nostdlib -Wl,-lc");
    }
    flags
});

static JOB_CLIENT: OnceLock<Option<jobserver::Client>> = OnceLock::new();

fn job_client() -> Option<&'static jobserver::Client> {
    JOB_CLIENT
        .get_or_init(|| unsafe { jobserver::Client::from_env() })
        .as_ref()
}

fn root_dir() -> PathBuf {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    out_dir.join("__root__")
}

fn tess_var(name: &str, default_value: impl AsRef<OsStr>) -> OsString {
    let name = format!("TESSERACT_{name}");
    var_os(name).unwrap_or_else(|| default_value.as_ref().to_owned())
}

fn is_musl() -> bool {
    var_os("CARGO_CFG_TARGET_ENV").as_deref() == Some(OsStr::new("musl"))
}

fn is_crt_static() -> bool {
    var_os("CARGO_CFG_TARGET_FEATURE")
        .map(|value| {
            value
                .as_encoded_bytes()
                .split(|byte| *byte == b',')
                .any(|slice| slice == b"crt-static")
        })
        .unwrap_or(false)
}

fn is_static_build() -> bool {
    is_musl() || is_crt_static()
}

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    if var_os("DOCS_RS").is_some() {
        fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/src/c_stub.rs"),
            out_dir.join("bindings.rs"),
        )
        .unwrap_display();
        return;
    }
    if var_os("TESSERACT_BUILD_FROM_SOURCE").is_some() {
        build_from_source();
    } else if !download_pre_built_binaries() {
        println!(
            "Pre-built archive for {:?} not found; building from source",
            std::env::var("TARGET")
        );
        build_from_source();
    }
    fs::write(out_dir.join("target"), std::env::var("TARGET").unwrap()).unwrap_display();
    fs::write(
        out_dir.join("version"),
        std::env::var("CARGO_PKG_VERSION").unwrap(),
    )
    .unwrap_display();
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=wrapper.h");
    println!("cargo::rerun-if-env-changed=PATH");
    generate_rust_bindings();
}

fn build_from_source() {
    for var in [
        "TESSERACT_CC",
        "TESSERACT_CXX",
        "TESSERACT_AR",
        "TESSERACT_RANLIB",
        "TESSERACT_CFLAGS",
        "TESSERACT_CXXFLAGS",
        "TESSERACT_LDFLAGS",
        "TESSERACT_BUILD_FROM_SOURCE",
        "TESSERACT_PRE_BUILT_ARCHIVE_URL",
        "TESSERACT_PRE_BUILT_ARCHIVE_HASH",
    ] {
        println!("cargo::rerun-if-env-changed={var}");
    }
    eprintln!("CC = {}", (*TESSERACT_CC).display());
    eprintln!("CXX = {}", (*TESSERACT_CXX).display());
    eprintln!("AR = {}", (*TESSERACT_AR).display());
    eprintln!("RANLIB = {}", (*TESSERACT_RANLIB).display());
    eprintln!("CFLAGS = {}", (*CFLAGS).display());
    eprintln!("CXXFLAGS = {}", (*CXXFLAGS).display());
    eprintln!("LDFLAGS = {}", (*LDFLAGS).display());
    if is_static_build() {
        build_musl();
    }
    build_libcxx();
    build_leptonica();
    build_tesseract();
}

fn download_pre_built_binaries() -> bool {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let target = std::env::var("TARGET").unwrap();
    let archive_file = out_dir.join("root.tar.zst");
    let (archive_url, archive_hash) = match (
        std::env::var("TESSERACT_PRE_BUILT_ARCHIVE_URL"),
        std::env::var("TESSERACT_PRE_BUILT_ARCHIVE_HASH"),
    ) {
        (Ok(url), Ok(hash)) => (url, hash),
        _ => {
            let Some((url, hash)) =
                PRE_BUILT_ARCHIVES
                    .iter()
                    .find_map(|(archive_target, url, hash)| {
                        (archive_target == &target).then_some((url, hash))
                    })
            else {
                return false;
            };
            (url.to_string(), hash.to_string())
        }
    };
    let mut existing_archive_is_ok = false;
    if archive_file.exists() {
        let actual_hash = b2sum(&archive_file);
        existing_archive_is_ok = archive_hash.as_str() == actual_hash.as_str();
        if !existing_archive_is_ok {
            let _ = fs::remove_file(&archive_file);
        }
    }
    if !existing_archive_is_ok {
        println!("Downloading pre-built archive from {archive_url:?}...");
        Command::new("curl")
            .args(["--location", "--fail"])
            .arg("-o")
            .arg(&archive_file)
            .arg(&archive_url)
            .status_checked()
            .unwrap_display();
        let actual_hash = b2sum(&archive_file);
        if archive_hash.as_str() != actual_hash.as_str() {
            panic!(
                "Failed to verify archive from {archive_url:?}:\n\
                expected hash {archive_hash:?}\n\
                actual hash   {actual_hash:?}\n"
            );
        }
    }
    let root_dir = out_dir.join("__root__");
    fs::create_dir_all(&root_dir).unwrap_display();
    Command::new("tar")
        .arg("-C")
        .arg(&root_dir)
        .arg("-xf")
        .arg(&archive_file)
        .status_checked()
        .unwrap_display();
    true
}

fn b2sum(file: &Path) -> String {
    let mut b2hasher = blake2b_simd::Params::new().hash_length(64).to_state();
    std::io::copy(&mut fs::File::open(file).unwrap_display(), &mut b2hasher).unwrap_display();
    b2hasher.finalize().to_hex().to_string()
}

fn generate_rust_bindings() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("__root__");
    println!("cargo:rustc-link-search={}/lib", root_dir.display());
    for lib in ["tesseract", "leptonica", "c++", "c++abi"] {
        println!("cargo:rustc-link-lib=static={lib}");
    }
    let builder = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-DNO_CONSOLE_IO")
        .clang_arg("-std=c23")
        .clang_arg(format!("-I{}/include", root_dir.display()))
        .parse_callbacks(Box::new(IgnoreComments))
        .allowlist_file("^.*/leptonica/.*$")
        .allowlist_file("^.*/tesseract/.*$");
    let bindings = builder.generate().unwrap_display();
    let out_path = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .unwrap_display()
}

#[derive(Debug)]
struct IgnoreComments;

impl ParseCallbacks for IgnoreComments {
    fn process_comment(&self, _comment: &str) -> Option<String> {
        Some(String::new())
    }
}

fn fetch_git(url: &str, tag: &str, dirname: &str) {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let dir = out_dir.join(dirname);
    if dir.exists() {
        return;
    }
    Command::new("git")
        .arg("clone")
        .arg(format!("--branch={tag}"))
        .arg("--depth=1")
        .arg(url)
        .arg(&dir)
        .status_checked()
        .unwrap_display();
}

fn hermetic_command(command: impl AsRef<OsStr>) -> Command {
    let mut command = Command::new(command);
    command.env_clear();
    if let Some(path) = var_os("PATH") {
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
    let root_dir = out_dir.join("__root__");
    let _ = fs::remove_dir_all(&build_dir);
    fs::create_dir_all(&build_dir).unwrap_display();
    configure(
        hermetic_command("cmake")
            .arg(format!("-DCMAKE_INSTALL_PREFIX={}", root_dir.display()))
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .arg("-DCMAKE_INSTALL_LIBDIR=lib")
            .arg(&archive_dir)
            .env("CC", &*TESSERACT_CC)
            .env("CXX", &*TESSERACT_CXX)
            .env("CFLAGS", &*CFLAGS)
            .env("CXXFLAGS", &*CXXFLAGS)
            .env("LDFLAGS", &*LDFLAGS)
            .env("PKG_CONFIG_PATH", root_dir.join("lib").join("pkgconfig"))
            .current_dir(&build_dir),
        &root_dir,
    )
    .status_checked()
    .unwrap_display();
}

fn make_with_cmake(build_dir: impl AsRef<Path>) {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("__root__");
    make_command()
        .arg("VERBOSE=1")
        .current_dir(&build_dir)
        .status_checked()
        .unwrap_display();
    hermetic_command("make")
        .arg("install")
        .current_dir(&build_dir)
        .status_checked()
        .unwrap_display();
    let _ = fs::remove_dir_all(root_dir.join("share").join("man"));
    let _ = fs::remove_dir_all(root_dir.join("share").join("doc"));
    fs::remove_dir_all(&build_dir).unwrap_display();
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
    fetch_git(
        "https://git.musl-libc.org/git/musl",
        &format!("v{MUSL_VERSION}"),
        &format!("musl-{MUSL_VERSION}"),
    );
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let archive_dir = out_dir.join(format!("musl-{MUSL_VERSION}"));
    let build_dir = archive_dir.join("__build__");
    let root_dir = out_dir.join("__root__");
    let _ = fs::remove_dir_all(&build_dir);
    fs::create_dir_all(&build_dir).unwrap_display();
    hermetic_command(archive_dir.join("configure"))
        .current_dir(&build_dir)
        .arg(format!("--prefix={}", root_dir.display()))
        .arg("--enable-wrapper=clang")
        .arg("--disable-shared")
        .env("AR", &*TESSERACT_AR)
        .env("RANLIB", &*TESSERACT_RANLIB)
        .env("CC", &*TESSERACT_CC)
        .env("CPPFLAGS", "-nostdinc")
        .env("CFLAGS", "-O3 -fPIC -fPIE")
        .env("LDFLAGS", "-fPIC -fPIE")
        .status_checked()
        .unwrap_display();
    make_with_cmake(&build_dir);
}

fn build_leptonica() {
    let dirname = format!("leptonica-{LEPTONICA_VERSION}");
    fetch_git(
        "https://github.com/DanBloomberg/leptonica",
        LEPTONICA_VERSION,
        &dirname,
    );
    build_with_cmake(&dirname, |command, _root_dir| {
        let more_c_flags = format!("{} -DNO_CONSOLE_IO", (*CFLAGS).display());
        command.env("CFLAGS", more_c_flags).args([
            "-DBUILD_SHARED_LIBS=0",
            "-DSTRICT_CONF=1",
            "-DENABLE_ZLIB=0",
            "-DENABLE_PNG=0",
            "-DENABLE_GIF=0",
            "-DENABLE_JPEG=0",
            "-DENABLE_TIFF=0",
            "-DENABLE_WEBP=0",
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
    .unwrap_display();
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
        let mut cxx_flags = OsString::new();
        cxx_flags.push(&*CXXFLAGS);
        cxx_flags.push(" -DNO_CONSOLE_IO");
        command.env("CXXFLAGS", cxx_flags).args([
            "-DBUILD_SHARED_LIBS=0",
            "-DGRAPHICS_DISABLED=1",
            "-DDISABLED_LEGACY_ENGINE=0",
            "-DBUILD_TRAINING_TOOLS=0",
            "-DBUILD_TESTS=0",
            "-DDISABLE_ARCHIVE=1",
            "-DDISABLE_CURL=1",
        ])
    });
    let _ = fs::remove_file(root_dir().join("bin").join("tesseract"));
    let _ = fs::remove_file(root_dir().join("share").join("tessdata"));
}

fn build_libcxx() {
    let dirname = format!("libcxx-{LIBCXX_VERSION}");
    fetch_git(
        "https://github.com/llvm/llvm-project",
        &format!("llvmorg-{LIBCXX_VERSION}"),
        &dirname,
    );
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let _ = fs::remove_dir_all(out_dir.join("__root__").join("include").join("c++"));
    build_with_cmake(
        Path::new(&dirname).join("libunwind"),
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
    let libcxx_cxx_flags = {
        let mut cxx_flags = OsString::new();
        cxx_flags.push(&*TESSERACT_CXXFLAGS);
        //if is_static_build() {
        //    cxx_flags.push("-nostdinc ");
        //}
        cxx_flags.push(" -nostdinc++ -O3 -fPIC -fPIE -I");
        cxx_flags.push(root_dir().join("include").join("c++").join("v1"));
        if is_static_build() {
            cxx_flags.push(" -isystem ");
            cxx_flags.push(root_dir().join("include"));
        }
        cxx_flags
    };
    configure_with_cmake(Path::new(&dirname).join("libcxx"), |command, _root_dir| {
        eprintln!("Override libcxx CXXFLAGS = {libcxx_cxx_flags:?}");
        if is_static_build() {
            command.arg("-DLIBCXX_HAS_MUSL_LIBC=1");
        }
        command.env("CXXFLAGS", &libcxx_cxx_flags).args([
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
        Path::new(&dirname).join("libcxxabi"),
        |command, _root_dir| {
            let mut cxx_flags = OsString::new();
            cxx_flags.push(&*TESSERACT_CXXFLAGS);
            //if is_static_build() {
            //    cxx_flags.push("-nostdinc ");
            //}
            cxx_flags.push(" -nostdinc++ -O3 -fPIC -fPIE -I");
            cxx_flags.push(
                out_dir
                    .join(&dirname)
                    .join("libcxx")
                    .join("__build__")
                    .join("include")
                    .join("c++")
                    .join("v1"),
            );
            cxx_flags.push(" -I");
            cxx_flags.push(out_dir.join(&dirname).join("libcxx").join("include"));
            if is_static_build() {
                cxx_flags.push(" -isystem ");
                cxx_flags.push(root_dir().join("include"));
            }
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
    build_with_cmake(Path::new(&dirname).join("libcxx"), |command, _root_dir| {
        eprintln!("Override libcxx CXXFLAGS = {libcxx_cxx_flags:?}");
        if is_static_build() {
            command.arg("-DLIBCXX_HAS_MUSL_LIBC=1");
        }
        command.env("CXXFLAGS", &libcxx_cxx_flags).args([
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
    let _ = fs::remove_file(root_dir().join("lib").join("libunwind.a"));
}

fn substitute(path: impl AsRef<Path>, rules: &[(impl AsRef<str>, impl AsRef<str>)]) {
    let mut text = fs::read_to_string(path.as_ref()).unwrap_display();
    for (value, replacement) in rules {
        text = text.replace(value.as_ref(), replacement.as_ref());
    }
    fs::write(path.as_ref(), text.as_bytes()).unwrap_display();
}

trait CommandExt {
    fn status_checked(&mut self) -> Result<ExitStatus, std::io::Error>;
}

impl CommandExt for Command {
    fn status_checked(&mut self) -> Result<ExitStatus, std::io::Error> {
        let status = self.status().map_err(|e| add_context(e, self))?;
        if !status.success() {
            let message = match status.code() {
                Some(code) => format!("Exited with status code {code}"),
                None => "Terminated by signal".to_string(),
            };
            return Err(add_context(std::io::Error::other(message), self));
        }
        Ok(status)
    }
}

fn add_context(error: std::io::Error, command: &Command) -> std::io::Error {
    let args = {
        let mut args = Vec::new();
        args.push(command.get_program());
        args.extend(command.get_args());
        args
    };
    let env: Vec<_> = command.get_envs().collect();
    let cwd = command
        .get_current_dir()
        .map(|path| path.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    std::io::Error::other(format!(
        "Failed to execute command: {error}\n\ncommand: {args:#?}\n\nenv: {env:#?}\n\ndir: {cwd:?}"
    ))
}

trait ResultExt<O> {
    fn unwrap_display(self) -> O;
}

impl<O, E: std::fmt::Display> ResultExt<O> for Result<O, E> {
    #[track_caller]
    fn unwrap_display(self) -> O {
        match self {
            Ok(o) => o,
            Err(e) => panic!("{e}"),
        }
    }
}
