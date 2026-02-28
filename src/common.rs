use core::ffi::CStr;
use core::ptr::NonNull;
use std::ffi::OsStr;
use std::path::Path;

use crate::OcrEngineMode;
use crate::PageSegmentationMode;
use crate::c;

/// Common tesseract methods.
pub struct Tesseract {
    pub(crate) ptr: NonNull<c::TessBaseAPI>,
}

impl Tesseract {
    /// Returns data directory.
    pub fn data_dir(&self) -> &Path {
        let ptr = unsafe { c::TessBaseAPIGetDatapath(self.ptr.as_ptr()) };
        assert!(!ptr.is_null());
        let c_str = unsafe { CStr::from_ptr(ptr) };
        let os_str = unsafe { OsStr::from_encoded_bytes_unchecked(c_str.to_bytes()) };
        Path::new(os_str)
    }

    /// Returns OCR engine mode.
    pub fn ocr_engine_mode(&self) -> OcrEngineMode {
        let ret = unsafe { c::TessBaseAPIOem(self.ptr.as_ptr()) };
        OcrEngineMode::from_raw(ret)
    }

    pub fn page_segmentation_mode(&self) -> PageSegmentationMode {
        let ret = unsafe { c::TessBaseAPIGetPageSegMode(self.ptr.as_ptr()) };
        PageSegmentationMode::from_raw(ret)
    }

    pub fn set_page_segmentation_mode(&mut self, mode: PageSegmentationMode) {
        unsafe { c::TessBaseAPISetPageSegMode(self.ptr.as_ptr(), mode as u32) };
    }

    pub fn set_source_resolution(&mut self, pixels_per_inch: u32) {
        unsafe { c::TessBaseAPISetSourceResolution(self.ptr.as_ptr(), pixels_per_inch as i32) };
    }

    pub fn set_min_orientation_margin(&mut self, margin: f64) {
        unsafe { c::TessBaseAPISetMinOrientationMargin(self.ptr.as_ptr(), margin) };
    }

    pub fn clear(&mut self) {
        unsafe { c::TessBaseAPIClear(self.ptr.as_ptr()) };
    }

    pub fn clear_cache(&mut self) {
        unsafe { c::TessBaseAPIClearPersistentCache(self.ptr.as_ptr()) };
    }

    pub fn clear_adaptive_classifier(&mut self) {
        unsafe { c::TessBaseAPIClearAdaptiveClassifier(self.ptr.as_ptr()) };
    }
}

impl Drop for Tesseract {
    fn drop(&mut self) {
        unsafe { c::TessBaseAPIEnd(self.ptr.as_ptr()) };
        unsafe { c::TessBaseAPIDelete(self.ptr.as_ptr()) };
    }
}
