#![doc = include_str!("../README.md")]

use core::ffi::CStr;
use core::ops::DerefMut;
use core::ptr::NonNull;
use std::ffi::OsStr;
use std::os::raw::c_void;
use std::path::Path;
use std::time::Duration;

mod c;
mod error;
mod image;
mod layout;
mod ocr;
mod types;
mod vars;
mod version;

pub use self::error::*;
pub use self::image::*;
pub use self::layout::*;
pub use self::ocr::*;
pub use self::types::*;
pub use self::version::*;

/// Common tesseract methods.
pub struct Tesseract {
    ptr: NonNull<c::TessBaseAPI>,
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

unsafe extern "C" fn cancel_callback<C: FnMut(i32) -> bool>(
    cancel_this: *mut c_void,
    words: i32,
) -> bool {
    let func: *mut C = cancel_this.cast();
    let func: &mut C = unsafe { core::mem::transmute(func) };
    func(words)
}

pub struct Monitor<C> {
    ptr: NonNull<c::ETEXT_DESC>,
    #[allow(unused)]
    cancel: Option<Box<C>>,
}

impl Monitor<()> {
    pub fn new() -> Self {
        let ptr = unsafe { c::TessMonitorCreate() };
        let ptr = NonNull::new(ptr).expect("TessMonitorCreate returned NULL");
        Self { ptr, cancel: None }
    }
}

impl<C: FnMut(i32) -> bool> Monitor<C> {
    pub fn with_cancel_callback(cancel: C) -> Self {
        let ptr = unsafe { c::TessMonitorCreate() };
        let ptr = NonNull::new(ptr).expect("TessMonitorCreate returned NULL");
        unsafe { c::TessMonitorSetCancelFunc(ptr.as_ptr(), Some(cancel_callback::<C>)) };
        let cancel = Box::new(cancel);
        let cancel_raw = Box::into_raw(cancel);
        unsafe { c::TessMonitorSetCancelThis(ptr.as_ptr(), cancel_raw as *mut c_void) };
        let cancel = Some(unsafe { Box::from_raw(cancel_raw) });
        Self { ptr, cancel }
    }

    pub fn get_cancel_callback(&mut self) -> &mut C {
        self.cancel
            .as_mut()
            .expect("Set in the constructor")
            .deref_mut()
    }
}

impl<C> Monitor<C> {
    pub fn set_progress_raw(&mut self, callback: c::TessProgressFunc) {
        unsafe { c::TessMonitorSetProgressFunc(self.ptr.as_ptr(), callback) }
    }

    pub fn get_progress(&self) -> i32 {
        unsafe { c::TessMonitorGetProgress(self.ptr.as_ptr()) }
    }

    pub fn set_deadline(&mut self, deadline: Duration) {
        let millis = deadline.as_millis().try_into().unwrap_or(i32::MAX);
        unsafe { c::TessMonitorSetDeadlineMSecs(self.ptr.as_ptr(), millis) };
    }
}

impl<C> Drop for Monitor<C> {
    fn drop(&mut self) {
        unsafe { c::TessMonitorDelete(self.ptr.as_ptr()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tesseract_works() {
        eprintln!("Tesseract: {:?}", version());
        eprintln!("Leptonica: {:?}", leptonica_version());
        eprintln!("Imagelib: {:?}", imagelib_versions());
        let mut tess = TextRecognizer::new().unwrap();
        eprintln!("Data: {:?}", tess.data_dir());
        println!("OCR engine mode: {:?}", tess.ocr_engine_mode());
        let image = Image::read_mem(include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/text.png"
        )))
        .unwrap();
        let results = tess.recognize_text(&image).unwrap();
        //let text = results.get_utf8_text();
        //eprintln!("Text: {text:?}");
        //eprintln!(
        //    "HOCR: {}",
        //    results.get_unlv_text().as_c_str().to_str().unwrap()
        //);
        //{
        //    let mut components = tess.components();
        //    while let Some(elem) = components.next(LayoutLevel::Word) {
        //        eprintln!(
        //            "Element: {:?}",
        //            elem.bounding_box(LayoutLevel::Word)
        //        );
        //    }
        //}
        {
            let mut results = results.iter();
            while let Some(elem) = results.next(LayoutLevel::Symbol) {
                eprintln!(
                    "Word: {:?}, bbox {:?}, font {:?}",
                    elem.get_utf8_text(LayoutLevel::Symbol),
                    elem.bounding_box(LayoutLevel::Symbol),
                    elem.word_font_attributes()
                );
                let mut choices = elem.choices();
                while let Some(choice) = choices.next() {
                    eprintln!("Choice: {:?}", choice.get_utf8_text())
                }
            }
        }
    }

    #[test]
    fn variables_to_markdown() {}
}
