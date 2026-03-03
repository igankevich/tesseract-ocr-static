use core::ffi::CStr;
use core::ptr::NonNull;
use std::os::raw::c_char;

macro_rules! define_enum {
    ($enum: ident
     $doc: literal
     $(($variant: ident $value: ident $variant_doc: literal $($deprecated: literal)*))+) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum $enum {
            $(
                #[doc = $variant_doc]
                $(#[deprecated = $deprecated])*
                $variant = c::$value as isize,
            )+
        }

        impl $enum {
            #[allow(unused)]
            #[allow(deprecated)]
            pub(crate) fn from_raw(value: u32) -> Self {
                match value {
                    $(c::$value => Self::$variant,)+
                    _ => panic!(concat!("Invalid ", stringify!($enum), " value")),
                }
            }
        }
    };
}

define_enum! {
    LayoutLevel "Element of layout hierarchy"
    (Block TessPageIteratorLevel_RIL_BLOCK "Block of text/image/separator line.")
    (Paragraph TessPageIteratorLevel_RIL_PARA "Paragraph within a block.")
    (TextLine TessPageIteratorLevel_RIL_TEXTLINE "Line within a paragraph.")
    (Word TessPageIteratorLevel_RIL_WORD "Word within a textline.")
    (Symbol TessPageIteratorLevel_RIL_SYMBOL "Symbol/character within a word.")
}

define_enum! {
    OcrEngineMode "OCR engine mode"
    (TesseractOnly TessOcrEngineMode_OEM_TESSERACT_ONLY "Run Tesseract only." "Use LstmOnly instead.")
    (LstmOnly TessOcrEngineMode_OEM_LSTM_ONLY "Run just the LSTM line recognizer.")
    (TesseractLstmCombined TessOcrEngineMode_OEM_TESSERACT_LSTM_COMBINED
     "Run LSTM, fall back to Tesseract when things get difficult." "Use LstmOnly instead.")
    (Default TessOcrEngineMode_OEM_DEFAULT "Choose mode automatically based on the configuration.")
}

define_enum! {
    PageSegmentationMode "Page layout analysis modes."
    (OsdOnly TessPageSegMode_PSM_OSD_ONLY "Orientation and script detection only.")
    (AutoOsd TessPageSegMode_PSM_AUTO_OSD "Automatic page segmentation with orientation and script detection (OSD).")
    (AutoOnly TessPageSegMode_PSM_AUTO_ONLY "Automatic page segmentation, but no OSD, or OCR.")
    (Auto TessPageSegMode_PSM_AUTO "Fully automatic page segmentation, but no OSD.")
    (SingleColumn TessPageSegMode_PSM_SINGLE_COLUMN "Assume a single column of text of variable sizes.")
    (SingleBlockVertText TessPageSegMode_PSM_SINGLE_BLOCK_VERT_TEXT "Assume a single uniform block of vertically aligned text.")
    (SingleBlock TessPageSegMode_PSM_SINGLE_BLOCK "Assume a single uniform block of text (default).")
    (SingleLine TessPageSegMode_PSM_SINGLE_LINE "Treat the image as a single text line.")
    (SingleWord TessPageSegMode_PSM_SINGLE_WORD "Treat the image as a single word.")
    (CircleWord TessPageSegMode_PSM_CIRCLE_WORD "Treat the image as a single word in a circle.")
    (SingleChar TessPageSegMode_PSM_SINGLE_CHAR "Treat the image as a single character.")
    (SparseText TessPageSegMode_PSM_SPARSE_TEXT "Find as much text as possible in no particular order.")
    (SparseTextOsd TessPageSegMode_PSM_SPARSE_TEXT_OSD "Sparse text with orientation and script det.")
    (RawLine TessPageSegMode_PSM_RAW_LINE "Treat the image as a single text line, bypassing hacks that are Tesseract-specific.")
}

#[allow(clippy::derivable_impls)]
impl Default for PageSegmentationMode {
    fn default() -> Self {
        Self::SingleBlock
    }
}

define_enum! {
    BlockType "Block type."
    (Unknown TessPolyBlockType_PT_UNKNOWN "Type is not yet known.")
    (FlowingText TessPolyBlockType_PT_FLOWING_TEXT "Text that lives inside a column.")
    (HeadingText TessPolyBlockType_PT_HEADING_TEXT "Text that spans more than one column.")
    (PulloutText TessPolyBlockType_PT_PULLOUT_TEXT "Text that is in a cross-column pull-out region.")
    (Equation TessPolyBlockType_PT_EQUATION "Partition belonging to an equation region.")
    (InlineEquation TessPolyBlockType_PT_INLINE_EQUATION "Partition has inline equation.")
    (Table TessPolyBlockType_PT_TABLE "Partition belonging to a table region.")
    (VerticalText TessPolyBlockType_PT_VERTICAL_TEXT "Text-line runs vertically.")
    (CaptionText TessPolyBlockType_PT_CAPTION_TEXT "Text that belongs to an image.")
    (FlowingImage TessPolyBlockType_PT_FLOWING_IMAGE "Image that lives inside a column.")
    (HeadingImage TessPolyBlockType_PT_HEADING_IMAGE "Image that spans more than one column.")
    (PulloutImage TessPolyBlockType_PT_PULLOUT_IMAGE "Image that is in a cross-column pull-out region.")
    (HorzLine TessPolyBlockType_PT_HORZ_LINE "Horizontal line.")
    (VertLine TessPolyBlockType_PT_VERT_LINE "Vertical line.")
    (Noise TessPolyBlockType_PT_NOISE "Lies outside of any column.")
}

define_enum! {
    Orientation "Text position on the page."
    (Up TessOrientation_ORIENTATION_PAGE_UP "Up")
    (Right TessOrientation_ORIENTATION_PAGE_RIGHT "Right")
    (Down TessOrientation_ORIENTATION_PAGE_DOWN "Down")
    (Left TessOrientation_ORIENTATION_PAGE_LEFT "Left")
}

define_enum! {
    WritingDirection "Text writing direction."
    (LeftToRight TessWritingDirection_WRITING_DIRECTION_LEFT_TO_RIGHT "Left-to-right")
    (RightToLeft TessWritingDirection_WRITING_DIRECTION_RIGHT_TO_LEFT "Right-to-left")
    (TopToBottom TessWritingDirection_WRITING_DIRECTION_TOP_TO_BOTTOM "Top-to-bottom")
}

define_enum! {
    TextlineOrder "The order of the lines of text."
    (LeftToRight TessTextlineOrder_TEXTLINE_ORDER_LEFT_TO_RIGHT "Left-to-right")
    (RightToLeft TessTextlineOrder_TEXTLINE_ORDER_RIGHT_TO_LEFT "Right-to-left")
    (TopToBottom TessTextlineOrder_TEXTLINE_ORDER_TOP_TO_BOTTOM "Top-to-bottom")
}

define_enum! {
    ParagraphJustification "Paragraph alignment."
    (Unknown TessParagraphJustification_JUSTIFICATION_UNKNOWN "The alignment is unclear.")
    (Left TessParagraphJustification_JUSTIFICATION_LEFT
     "Each line, except possibly the first, is flush to the same left tab stop.")
    (Center TessParagraphJustification_JUSTIFICATION_CENTER
     "The text lines of the paragraph are centered about a line going down through their middle of the text lines.")
    (Right TessParagraphJustification_JUSTIFICATION_RIGHT
     "Each line, except possibly the first, is flush to the same right tab stop.")
}

/// Rectangle.
///
/// Coordinate system has the origin at left-top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rectangle {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

/// Line.
///
/// Coordinate system has the origin at left-top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub x1: u32,
    pub y1: u32,
    pub x2: u32,
    pub y2: u32,
}

/// Orientation.
#[derive(Debug, Clone)]
pub struct OrientationParams {
    /// Text position on the page.
    pub orientation: Orientation,
    /// How text is aligned.
    pub writing_direction: WritingDirection,
    /// How lines are aligned.
    pub textline_order: TextlineOrder,
    /// Counter-clockwise rotation angle in radians; the range is _[π/4; π/4]_.
    ///
    /// Rotation the block by this angle makes the text upright.
    pub deskew_angle: f32,
}

/// Paragraph information.
#[derive(Debug, Clone)]
pub struct ParagraphInfo {
    /// Paragraph alignment.
    pub justification: ParagraphJustification,
    /// Whether we believe this is a member of an ordered or unordered list or not.
    pub is_list_item: bool,
    /// Whether the first line of the paragraph is aligned with the other
    /// lines of the paragraph even though subsequent paragraphs have first
    /// line indents.  This typically indicates that this is the continuation
    /// of a previous paragraph or that it is the very first paragraph in
    /// the chapter.
    pub is_crown: bool,
    /// First line indentation with respect to `justification`.
    ///
    /// Can be negative.
    pub first_line_indent: i32,
}

/// Font attributes.
#[derive(Debug, Clone)]
pub struct FontAttrs {
    pub is_bold: bool,
    pub is_italic: bool,
    pub is_underlined: bool,
    pub is_monospace: bool,
    pub is_serif: bool,
    pub is_smallcaps: bool,
    pub point_size: u32,
    pub font_id: i32,
}

/// Tesseract-specific string.
pub struct Text {
    pub(crate) ptr: NonNull<c_char>,
}

impl Text {
    /// Returns text as C string.
    pub fn as_c_str(&self) -> &CStr {
        unsafe { CStr::from_ptr(self.ptr.as_ptr()) }
    }
}

impl Drop for Text {
    fn drop(&mut self) {
        unsafe { c::TessDeleteText(self.ptr.as_ptr()) };
    }
}

impl AsRef<CStr> for Text {
    fn as_ref(&self) -> &CStr {
        self.as_c_str()
    }
}

impl core::fmt::Debug for Text {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_c_str(), f)
    }
}

/// Tesseract-specific UTF-8 string.
pub struct Utf8Text(pub(crate) Text);

impl Utf8Text {
    /// Returns text as UTF-8 string.
    pub fn as_str(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(self.0.as_c_str().to_bytes()) }
    }
}

impl AsRef<str> for Utf8Text {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl core::fmt::Debug for Utf8Text {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        core::fmt::Debug::fmt(self.as_str(), f)
    }
}
