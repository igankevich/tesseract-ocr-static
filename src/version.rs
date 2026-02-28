use core::ffi::CStr;

use crate::c;

/// Returns Tesseract library version.
///
/// ```rust
/// assert_eq!("5.5.2", rustyract::version());
/// ```
pub fn version() -> &'static str {
    let c_str = unsafe { CStr::from_ptr(c::TessVersion()) };
    unsafe { core::str::from_utf8_unchecked(c_str.to_bytes()) }
}

/// Returns Leptonica library version.
///
/// ```rust
/// assert_eq!("leptonica-1.87.0", rustyract::leptonica_version());
/// ```
pub fn leptonica_version() -> &'static str {
    let c_str = unsafe { CStr::from_ptr(c::getLeptonicaVersion()) };
    unsafe { core::str::from_utf8_unchecked(c_str.to_bytes()) }
}

/// Returns versions of the dependencies.
///
/// ```rust
/// assert_eq!(
///     "libjpeg 6b (libjpeg-turbo 3.1.3) : libpng 1.6.55 : libtiff 4.7.1 : \
///     zlib 1.3.2 : libwebp 1.6.0",
///     rustyract::imagelib_versions()
/// );
/// ```
pub fn imagelib_versions() -> &'static str {
    let c_str = unsafe { CStr::from_ptr(c::getImagelibVersions()) };
    unsafe { core::str::from_utf8_unchecked(c_str.to_bytes()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_version_works() {
        assert_eq!(
            "5.5.2",
            unsafe { CStr::from_ptr(c::TessVersion()) }
                .to_str()
                .unwrap()
        );
        assert_eq!(
            "leptonica-1.87.0",
            unsafe { CStr::from_ptr(c::getLeptonicaVersion()) }
                .to_str()
                .unwrap()
        );
        assert_eq!(
            "libjpeg 6b (libjpeg-turbo 3.1.3) : libpng 1.6.55 : libtiff 4.7.1 : zlib 1.3.2 : libwebp 1.6.0",
            unsafe { CStr::from_ptr(c::getImagelibVersions()) }
                .to_str()
                .unwrap()
        );
    }
}
