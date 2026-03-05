use core::ffi::CStr;

/// Returns Tesseract library version.
///
/// ```rust
/// assert_eq!("5.5.2", tesseract_ocr_static::version());
/// ```
pub fn version() -> &'static str {
    let c_str = unsafe { CStr::from_ptr(c::TessVersion()) };
    unsafe { core::str::from_utf8_unchecked(c_str.to_bytes()) }
}

/// Returns Leptonica library version.
///
/// ```rust
/// assert_eq!("leptonica-1.87.0", tesseract_ocr_static::leptonica_version());
/// ```
pub fn leptonica_version() -> &'static str {
    let c_str = unsafe { CStr::from_ptr(c::getLeptonicaVersion()) };
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
    }
}
