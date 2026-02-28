use core::ffi::CStr;
use core::marker::PhantomData;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ptr::NonNull;

use crate::Element;
use crate::FontAttrs;
use crate::Image;
use crate::InitFailed;
use crate::LayoutIter;
use crate::LayoutLevel;
use crate::OcrEngineMode;
use crate::RecognitionFailed;
use crate::Rectangle;
use crate::Tesseract;
use crate::Text;
use crate::Utf8Text;
use crate::c;

const ENGLISH: &CStr = c"eng";

/// OCR configuration.
#[derive(Debug)]
pub struct Config<'a, 'b> {
    /// Data directory where language-specific training data is stored.
    ///
    /// If not specified, the `TESSDATA_PREFIX` environment variable is used instead.
    /// If the variable isn't defined, the build-time default is used.
    pub data_dir: Option<&'a CStr>,
    /// Languages are specified by their three-letter ISO codes separated by '+' symbol.
    ///
    /// English is the default.
    pub languages: &'b CStr,
    /// OCR engine mode. LSTM is the default.
    pub ocr_engine_mode: OcrEngineMode,
}

impl Default for Config<'static, 'static> {
    fn default() -> Self {
        Self {
            data_dir: None,
            languages: ENGLISH,
            ocr_engine_mode: OcrEngineMode::LstmOnly,
        }
    }
}

/// OCR engine interface.
pub struct TextRecognizer {
    base: Tesseract,
}

impl TextRecognizer {
    /// Creates new text recognizer with the default data directory, default language (English),
    /// and default OCR engine mode (LSTM).
    pub fn new() -> Result<Self, InitFailed> {
        Self::with_languages(ENGLISH)
    }

    /// Creates new text recognizer with the specified languages.
    ///
    /// Languages are specified by their three-letter ISO codes separated by '+' symbol.
    pub fn with_languages(languages: &CStr) -> Result<Self, InitFailed> {
        Self::with_config(Config {
            languages,
            ..Default::default()
        })
    }

    /// Creates new text recognizer with the provided configuration.
    pub fn with_config(config: Config<'_, '_>) -> Result<Self, InitFailed> {
        let ptr = unsafe { c::TessBaseAPICreate() };
        let ptr = NonNull::new(ptr).expect("TessBaseAPICreate returned NULL");
        let ret = unsafe {
            c::TessBaseAPIInit2(
                ptr.as_ptr(),
                config
                    .data_dir
                    .map(|x| x.as_ptr())
                    .unwrap_or(core::ptr::null_mut()),
                config.languages.as_ptr(),
                config.ocr_engine_mode as u32,
            )
        };
        if ret < 0 {
            return Err(InitFailed);
        }
        let base = Tesseract { ptr };
        Ok(Self { base })
    }

    /// Recognizes text in the provided image and returns an iterator over the results.
    pub fn recognize_text<'a>(
        &'a mut self,
        image: &Image,
    ) -> Result<RecognitionResults<'a>, RecognitionFailed> {
        unsafe { c::TessBaseAPISetImage2(self.as_ptr(), image.ptr.as_ptr()) };
        let ret = unsafe { c::TessBaseAPIRecognize(self.as_ptr(), core::ptr::null_mut()) };
        if ret < 0 {
            return Err(RecognitionFailed);
        }
        Ok(RecognitionResults { inner: self })
    }

    /// Recognizes text in the specified rectangle of the provided image and
    /// returns an iterator over the results.
    pub fn recognize_text_in_rect<'a>(
        &'a mut self,
        image: &Image,
        rect: &Rectangle,
    ) -> Result<RecognitionResults<'a>, RecognitionFailed> {
        unsafe { c::TessBaseAPISetImage2(self.as_ptr(), image.ptr.as_ptr()) };
        unsafe {
            c::TessBaseAPISetRectangle(
                self.as_ptr(),
                rect.left as i32,
                rect.top as i32,
                rect.width as i32,
                rect.height as i32,
            )
        };
        let ret = unsafe { c::TessBaseAPIRecognize(self.as_ptr(), core::ptr::null_mut()) };
        if ret < 0 {
            return Err(RecognitionFailed);
        }
        Ok(RecognitionResults { inner: self })
    }

    /// Analyzes the text layout in the provided image and returns layout analysis results as an
    /// iterator.
    ///
    /// If you only need layout, consider using [`LayoutAnalyzer`](crate::LayoutAnalyzer) that uses
    /// less memory.
    pub fn analyze_layout<'a>(&'a self, image: &Image) -> LayoutIter<'a> {
        unsafe { c::TessBaseAPISetImage2(self.as_ptr(), image.ptr.as_ptr()) };
        let ptr = unsafe { c::TessBaseAPIAnalyseLayout(self.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIAnalyseLayout returned NULL");
        unsafe { c::TessPageIteratorBegin(ptr.as_ptr()) };
        LayoutIter {
            ptr,
            phantom: PhantomData,
        }
    }

    /// Returns the number of Directed Acyclic Word Graph (DAWG) in the dictionary.
    pub fn num_dawgs(&self) -> u32 {
        let ret = unsafe { c::TessBaseAPIGetPageSegMode(self.ptr.as_ptr()) };
        ret as u32
    }

    #[inline]
    fn as_ptr(&self) -> *mut c::TessBaseAPI {
        self.base.ptr.as_ptr()
    }
}

impl Deref for TextRecognizer {
    type Target = Tesseract;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for TextRecognizer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// Text recognition results.
pub struct RecognitionResults<'a> {
    inner: &'a TextRecognizer,
}

impl<'a> RecognitionResults<'a> {
    /// Returns recognized text as string.
    pub fn get_utf8_text(&self) -> Utf8Text {
        let ptr = unsafe { c::TessBaseAPIGetUTF8Text(self.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIGetUTF8Text returned NULL");
        Utf8Text(Text { ptr })
    }

    /// Returns recognized text as HTML-formatted string with
    /// [hOCR markup](https://en.wikipedia.org/wiki/HOCR).
    ///
    /// `page` is zero-based page index that appears in the output as one-based.
    pub fn get_hocr_text(&self, page: u32) -> Text {
        let ptr = unsafe { c::TessBaseAPIGetHOCRText(self.as_ptr(), page as i32) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIGetHOCRText returned NULL");
        Text { ptr }
    }

    /// Returns recognized text as XML-formatted string with
    /// [ALTO markup](https://en.wikipedia.org/wiki/Analyzed_Layout_and_Text_Object).
    ///
    /// `page` is zero-based page index that appears in the output.
    pub fn get_alto_text(&self, page: u32) -> Text {
        let ptr = unsafe { c::TessBaseAPIGetAltoText(self.as_ptr(), page as i32) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIGetAltoText returned NULL");
        Text { ptr }
    }

    /// Returns recognized text as XML-formatted string with PAGE markup.
    ///
    /// `page` is zero-based page index that appears in the output as one-based.
    ///
    /// WARNING: This function is currently broken (throws an exception).
    #[doc(hidden)]
    pub fn get_page_text(&self, page: u32) -> Text {
        let ptr = unsafe { c::TessBaseAPIGetPAGEText(self.as_ptr(), page as i32) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIGetPAGEText returned NULL");
        Text { ptr }
    }

    /// Returns recognized text as TSV-formatted string.
    ///
    /// `page` is zero-based page index that appears in the output as one-based.
    ///
    /// # TSV columns
    ///
    /// | Column | Comment |
    /// |--------|---------|
    /// | Page number | |
    /// | Block index | |
    /// | Paragraph index | |
    /// | Line index | |
    /// | Word index | |
    /// | Bounding box left | |
    /// | Bounding box top | |
    /// | Bounding box width | |
    /// | Bounding box height | |
    /// | Confidence | `-1` means end of the element |
    /// | Word | |
    pub fn get_tsv_text(&self, page: u32) -> Text {
        let ptr = unsafe { c::TessBaseAPIGetTsvText(self.as_ptr(), page as i32) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIGetTsvText returned NULL");
        Text { ptr }
    }

    /// Returns the
    /// [box file](https://tesseract-ocr.github.io/tessdoc/tess4/Make-Box-Files.html)
    /// for the page.
    ///
    /// `page` is zero-based page index that appears in the output.
    pub fn get_box_text(&self, page: u32) -> Text {
        let ptr = unsafe { c::TessBaseAPIGetBoxText(self.as_ptr(), page as i32) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIGetBoxText returned NULL");
        Text { ptr }
    }

    /// Returns the
    /// [LSTM box file](https://tesseract-ocr.github.io/tessdoc/tess4/Make-Box-Files.html)
    /// for the page.
    ///
    /// `page` is zero-based page index that appears in the output.
    pub fn get_lstm_box_text(&self, page: u32) -> Text {
        let ptr = unsafe { c::TessBaseAPIGetLSTMBoxText(self.as_ptr(), page as i32) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIGetLSTMBoxText returned NULL");
        Text { ptr }
    }

    /// Returns the
    /// [WordStr box file](https://tesseract-ocr.github.io/tessdoc/tess4/Make-Box-Files.html)
    /// for the page.
    ///
    /// `page` is zero-based page index that appears in the output.
    pub fn get_word_str_box_text(&self, page: u32) -> Text {
        let ptr = unsafe { c::TessBaseAPIGetWordStrBoxText(self.as_ptr(), page as i32) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIGetWordStrBoxText returned NULL");
        Text { ptr }
    }

    /// Returns recognized text as UNLV-formatted string.
    pub fn get_unlv_text(&self) -> Text {
        let ptr = unsafe { c::TessBaseAPIGetUNLVText(self.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIGetUNLVText returned NULL");
        Text { ptr }
    }

    /// Returns an iterator over text elements.
    pub fn iter(&self) -> ResultIter<'a> {
        let ptr = unsafe { c::TessBaseAPIGetIterator(self.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIGetIterator returned NULL");
        ResultIter {
            ptr,
            phantom: PhantomData,
        }
    }

    /// Returns a copy of the thresholded image.
    pub fn get_thresholded_image(&self) -> Image {
        let ptr = unsafe { c::TessBaseAPIGetThresholdedImage(self.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIGetThresholdedImage returned NULL");
        Image { ptr }
    }

    /// Returns thresholded image scale.
    pub fn get_thresholded_image_scale_factor(&self) -> u32 {
        let ret = unsafe { c::TessBaseAPIGetThresholdedImageScaleFactor(self.as_ptr()) };
        ret as u32
    }

    /// Returns average gradient of lines on page.
    pub fn get_gradient(&self) -> f32 {
        unsafe { c::TessBaseAPIGetGradient(self.as_ptr()) }
    }

    /// Returns `true` if the word is valid according to Tesseract's language model.
    #[doc(hidden)]
    pub fn is_valid_word(&self, word: &CStr) -> bool {
        let ret = unsafe { c::TessBaseAPIIsValidWord(self.as_ptr(), word.as_ptr()) };
        ret != 0
    }

    /// Returns text direction in Tesseract's coordinates.
    #[doc(hidden)]
    pub fn get_text_direction(&self) -> Option<(u32, f32)> {
        let mut offset: i32 = 0;
        let mut slope: f32 = 0.0;
        let ret = unsafe { c::TessBaseAPIGetTextDirection(self.as_ptr(), &mut offset, &mut slope) };
        if ret == 0 {
            return None;
        }
        Some((offset as u32, slope))
    }

    #[inline]
    fn as_ptr(&self) -> *mut c::TessBaseAPI {
        self.inner.as_ptr()
    }
}

/// An iterator over recognition results.
pub struct ResultIter<'a> {
    ptr: NonNull<c::TessResultIterator>,
    #[allow(unused)]
    phantom: PhantomData<&'a Tesseract>,
}

impl<'a> ResultIter<'a> {
    /// Returns the next text element at the specified level or
    /// `None` if such an element doesn't exist.
    #[must_use]
    pub fn next(&mut self, level: LayoutLevel) -> Option<TextElement<'_>> {
        let ret = unsafe { c::TessResultIteratorNext(self.ptr.as_ptr(), level as u32) };
        (ret != 0).then_some(TextElement { iter: self })
    }

    /// Returnrs an iterator over layout elements.
    pub fn as_layout_iter(&self) -> LayoutIter<'a> {
        let ptr = unsafe { c::TessResultIteratorGetPageIterator(self.ptr.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("TessResultIteratorGetPageIterator returned NULL");
        LayoutIter {
            ptr,
            phantom: PhantomData,
        }
    }
}

impl Drop for ResultIter<'_> {
    fn drop(&mut self) {
        unsafe { c::TessResultIteratorDelete(self.ptr.as_ptr()) };
    }
}

impl Clone for ResultIter<'_> {
    fn clone(&self) -> Self {
        let ptr = unsafe { c::TessResultIteratorCopy(self.ptr.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("TessResultIteratorCopy returned NULL");
        Self {
            ptr,
            phantom: PhantomData,
        }
    }
}

/// Text element.
///
/// A layout element with the recognized text.
pub struct TextElement<'a> {
    iter: &'a ResultIter<'a>,
}

impl<'a> TextElement<'a> {
    /// Get recognized text as UTF-8 string.
    pub fn get_utf8_text(&self, level: LayoutLevel) -> Utf8Text {
        let ptr = unsafe { c::TessResultIteratorGetUTF8Text(self.iter.ptr.as_ptr(), level as u32) };
        let ptr = NonNull::new(ptr).expect("TessResultIteratorGetUTF8Text returned NULL");
        Utf8Text(Text { ptr })
    }

    /// Returns the mean confidence of the element at the given level.
    ///
    /// The confidence range is _[0; 100]_.
    pub fn confidence(&self, level: LayoutLevel) -> f32 {
        unsafe { c::TessResultIteratorConfidence(self.iter.ptr.as_ptr(), level as u32) }
    }

    /// Returns the language that was used to recognize the word.
    pub fn word_recognition_language(&self) -> Option<&CStr> {
        let ptr = unsafe { c::TessResultIteratorWordRecognitionLanguage(self.iter.ptr.as_ptr()) };
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { CStr::from_ptr(ptr) })
    }

    /// Returns `true` if the current word is a dictionary word.
    pub fn word_is_from_dictionary(&self) -> bool {
        let ret = unsafe { c::TessResultIteratorWordIsFromDictionary(self.iter.ptr.as_ptr()) };
        ret != 0
    }

    /// Returns true if the current word is a number.
    pub fn word_is_numeric(&self) -> bool {
        let ret = unsafe { c::TessResultIteratorWordIsNumeric(self.iter.ptr.as_ptr()) };
        ret != 0
    }

    /// Returns font attributes of the current word as well as the font name.
    pub fn word_font_attributes(&self) -> Option<(FontAttrs, &CStr)> {
        let mut is_bold = 0;
        let mut is_italic = 0;
        let mut is_underlined = 0;
        let mut is_monospace = 0;
        let mut is_serif = 0;
        let mut is_smallcaps = 0;
        let mut point_size = 0;
        let mut font_id = 0;
        let ptr = unsafe {
            c::TessResultIteratorWordFontAttributes(
                self.iter.ptr.as_ptr(),
                &mut is_bold,
                &mut is_italic,
                &mut is_underlined,
                &mut is_monospace,
                &mut is_serif,
                &mut is_smallcaps,
                &mut point_size,
                &mut font_id,
            )
        };
        if ptr.is_null() {
            return None;
        }
        let font = unsafe { CStr::from_ptr(ptr) };
        let attrs = FontAttrs {
            is_bold: is_bold != 0,
            is_italic: is_italic != 0,
            is_underlined: is_underlined != 0,
            is_monospace: is_monospace != 0,
            is_serif: is_serif != 0,
            is_smallcaps: is_smallcaps != 0,
            point_size: point_size as u32,
            font_id,
        };
        Some((attrs, font))
    }

    /// Returns `true` if the current symbol is a superscript.
    pub fn symbol_is_superscript(&self) -> bool {
        let ret = unsafe { c::TessResultIteratorSymbolIsSuperscript(self.iter.ptr.as_ptr()) };
        ret != 0
    }

    /// Returns `true` if the current symbol is a subscript.
    pub fn symbol_is_subscript(&self) -> bool {
        let ret = unsafe { c::TessResultIteratorSymbolIsSubscript(self.iter.ptr.as_ptr()) };
        ret != 0
    }

    /// Returns `true` if the current symbol is a dropcap.
    pub fn symbol_is_dropcap(&self) -> bool {
        let ret = unsafe { c::TessResultIteratorSymbolIsDropcap(self.iter.ptr.as_ptr()) };
        ret != 0
    }

    /// Returns an iteratove over classifier choices for the current symbol.
    pub fn choices(&self) -> ChoiceIterator<'a> {
        let ptr = unsafe { c::TessResultIteratorGetChoiceIterator(self.iter.ptr.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("TessResultIteratorGetChoiceIterator returned NULL");
        ChoiceIterator {
            ptr,
            results: self.iter,
        }
    }
}

impl<'a> Deref for TextElement<'a> {
    type Target = Element<'a>;

    fn deref(&self) -> &Self::Target {
        unsafe { core::mem::transmute(self) }
    }
}

/// A symbol choice.
pub struct ClassifierChoice<'a> {
    iter: &'a ChoiceIterator<'a>,
}

impl ClassifierChoice<'_> {
    /// Returns the choice as UTF-8 string.
    pub fn get_utf8_text(&self) -> &str {
        let ptr = unsafe { c::TessChoiceIteratorGetUTF8Text(self.iter.ptr.as_ptr()) };
        assert!(!ptr.is_null());
        let c_str = unsafe { CStr::from_ptr(ptr) };
        unsafe { core::str::from_utf8_unchecked(c_str.to_bytes()) }
    }

    /// Returns the confidence in the range of _[0; 100]_.
    pub fn confidence(&self) -> f32 {
        unsafe { c::TessChoiceIteratorConfidence(self.iter.ptr.as_ptr()) }
    }
}

impl AsRef<str> for ClassifierChoice<'_> {
    fn as_ref(&self) -> &str {
        self.get_utf8_text()
    }
}

/// An iterator over classifier choices for a symbol.
pub struct ChoiceIterator<'a> {
    ptr: NonNull<c::TessChoiceIterator>,
    #[allow(unused)]
    results: &'a ResultIter<'a>,
}

impl ChoiceIterator<'_> {
    /// Returns the next choice.
    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<ClassifierChoice<'_>> {
        let ret = unsafe { c::TessChoiceIteratorNext(self.ptr.as_ptr()) };
        (ret != 0).then_some(ClassifierChoice { iter: self })
    }
}

impl Drop for ChoiceIterator<'_> {
    fn drop(&mut self) {
        unsafe { c::TessChoiceIteratorDelete(self.ptr.as_ptr()) };
    }
}
