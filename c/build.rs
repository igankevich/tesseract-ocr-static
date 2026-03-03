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
    if is_musl_target() {
        flags.push(" --sysroot ");
        flags.push(root_dir());
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
    flags
});

static LDFLAGS: LazyLock<OsString> = LazyLock::new(|| {
    let mut flags = OsString::new();
    flags.push(&*TESSERACT_LDFLAGS);
    flags.push(" ");
    flags.push(COMMON_LDFLAGS);
    flags.push(" -Wl,-L");
    flags.push(root_dir().join("lib"));
    if is_musl_target() {
        flags.push(" -nostdlib -Wl,-lc --sysroot ");
        flags.push(root_dir());
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
    out_dir.join("root")
}

fn tess_var(name: &str, default_value: impl AsRef<OsStr>) -> OsString {
    let name = format!("TESSERACT_{name}");
    var_os(name).unwrap_or_else(|| default_value.as_ref().to_owned())
}

fn is_musl_target() -> bool {
    var_os("CARGO_CFG_TARGET_ENV").as_deref() == Some(OsStr::new("musl"))
}

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=wrapper.h");
    println!("cargo::rerun-if-env-changed=PATH");
    println!("cargo::rerun-if-env-changed=TESSERACT_CC");
    println!("cargo::rerun-if-env-changed=TESSERACT_CXX");
    println!("cargo::rerun-if-env-changed=TESSERACT_AR");
    println!("cargo::rerun-if-env-changed=TESSERACT_RANLIB");
    println!("cargo::rerun-if-env-changed=TESSERACT_CFLAGS");
    println!("cargo::rerun-if-env-changed=TESSERACT_CXXFLAGS");
    println!("cargo::rerun-if-env-changed=TESSERACT_LDFLAGS");
    eprintln!("CC = {}", (*TESSERACT_CC).display());
    eprintln!("CXX = {}", (*TESSERACT_CXX).display());
    eprintln!("AR = {}", (*TESSERACT_AR).display());
    eprintln!("RANLIB = {}", (*TESSERACT_RANLIB).display());
    eprintln!("CFLAGS = {}", (*CFLAGS).display());
    eprintln!("CXXFLAGS = {}", (*CXXFLAGS).display());
    eprintln!("LDFLAGS = {}", (*LDFLAGS).display());
    if is_musl_target() {
        build_musl();
    }
    build_libcxx();
    build_leptonica();
    build_tesseract();
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let root_dir = out_dir.join("root");
    println!("cargo:rustc-link-search={}/lib", root_dir.display());
    println!("cargo:rustc-link-lib=static=tesseract");
    println!("cargo:rustc-link-lib=static=leptonica");
    println!("cargo:rustc-link-lib=static=c++");
    println!("cargo:rustc-link-lib=static=c++abi");
    let builder = bindgen::Builder::default()
        .header("wrapper.h")
        .clang_arg("-DNO_CONSOLE_IO")
        .clang_arg("-std=c23")
        .clang_arg(format!("-I{}/include", root_dir.display()))
        .parse_callbacks(Box::new(IgnoreComments));
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
    let root_dir = out_dir.join("root");
    let _ = fs::remove_dir_all(&build_dir);
    fs::create_dir_all(&build_dir).unwrap();
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
    fetch_git(
        "https://git.musl-libc.org/git/musl",
        &format!("v{MUSL_VERSION}"),
        &format!("musl-{MUSL_VERSION}"),
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
        .arg("--disable-shared")
        .env("AR", &*TESSERACT_AR)
        .env("RANLIB", &*TESSERACT_RANLIB)
        .env("CC", &*TESSERACT_CC)
        .env("CPPFLAGS", "-nostdinc")
        .env("CFLAGS", "-O3 -fPIC -fPIE")
        .env("LDFLAGS", "-fPIC -fPIE")
        .status_checked()
        .unwrap();
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
    configure_with_cmake(Path::new(&dirname).join("libcxx"), |command, root_dir| {
        let cxx_flags = format!(
            "{} -nostdinc++ {} -I{}",
            if is_musl_target() { "-nostdinc" } else { "" },
            (*CXXFLAGS).display(),
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
        Path::new(&dirname).join("libcxxabi"),
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
                (*CXXFLAGS).display(),
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
    build_with_cmake(Path::new(&dirname).join("libcxx"), |command, root_dir| {
        let cxx_flags = format!(
            "{} -nostdinc++ {} -I{}",
            if is_musl_target() { "-nostdinc" } else { "" },
            (*CXXFLAGS).display(),
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
        "Failed to execute {args:?}: {error}; env {env:?}, dir {cwd:?}"
    ))
}
