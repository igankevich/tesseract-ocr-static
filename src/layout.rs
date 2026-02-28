use core::marker::PhantomData;
use core::ops::Deref;
use core::ops::DerefMut;
use core::ptr::NonNull;

use crate::c;
use crate::BlockType;
use crate::Image;
use crate::LayoutLevel;
use crate::Line;
use crate::Orientation;
use crate::OrientationParams;
use crate::ParagraphInfo;
use crate::ParagraphJustification;
use crate::Rectangle;
use crate::Tesseract;
use crate::TextlineOrder;
use crate::WritingDirection;

/// Layout analysis engine.
///
/// Uses less memory than [`TextRecognizer`](crate::TextRecognizer) but can't recognize text, only
/// analyze its layout.
pub struct LayoutAnalyzer {
    base: Tesseract,
}

impl LayoutAnalyzer {
    /// Creates new layout analyzer with default parameters.
    pub fn new() -> Self {
        let ptr = unsafe { c::TessBaseAPICreate() };
        let ptr = NonNull::new(ptr).expect("TessBaseAPICreate returned NULL");
        unsafe { c::TessBaseAPIInitForAnalysePage(ptr.as_ptr()) };
        let base = Tesseract { ptr };
        Self { base }
    }

    /// Analyzes text layout in the provided image and returns an iterator over the results.
    pub fn analyze(&mut self, image: &Image) -> LayoutIter<'_> {
        unsafe { c::TessBaseAPISetImage2(self.as_ptr(), image.ptr.as_ptr()) };
        let ptr = unsafe { c::TessBaseAPIAnalyseLayout(self.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("TessBaseAPIAnalyseLayout returned NULL");
        unsafe { c::TessPageIteratorBegin(ptr.as_ptr()) };
        LayoutIter {
            ptr,
            phantom: PhantomData,
        }
    }

    #[inline]
    fn as_ptr(&self) -> *mut c::TessBaseAPI {
        self.base.ptr.as_ptr()
    }
}

impl Deref for LayoutAnalyzer {
    type Target = Tesseract;

    fn deref(&self) -> &Self::Target {
        &self.base
    }
}

impl DerefMut for LayoutAnalyzer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.base
    }
}

/// An iterator over text layout elements.
pub struct LayoutIter<'a> {
    pub(crate) ptr: NonNull<c::TessPageIterator>,
    #[allow(unused)]
    pub(crate) phantom: PhantomData<&'a Tesseract>,
}

impl<'a> LayoutIter<'a> {
    /// Returns the next layout element at the specified level or
    /// `None` if such an element doesn't exist.
    #[must_use]
    pub fn next(&mut self, level: LayoutLevel) -> Option<Element<'_>> {
        let ret = unsafe { c::TessPageIteratorNext(self.ptr.as_ptr(), level as u32) };
        (ret != 0).then_some(Element { iter: self })
    }
}

impl Drop for LayoutIter<'_> {
    fn drop(&mut self) {
        unsafe { c::TessPageIteratorDelete(self.ptr.as_ptr()) };
    }
}

impl Clone for LayoutIter<'_> {
    fn clone(&self) -> Self {
        let ptr = unsafe { c::TessPageIteratorCopy(self.ptr.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("TessPageIteratorCopy returned NULL");
        Self {
            ptr,
            phantom: PhantomData,
        }
    }
}

/// Text layout element.
pub struct Element<'a> {
    iter: &'a LayoutIter<'a>,
}

impl Element<'_> {
    /// Returns `true` if the element is at the start of the given level.
    pub fn is_at_beginning_of(&self, level: LayoutLevel) -> bool {
        let ret =
            unsafe { c::TessPageIteratorIsAtBeginningOf(self.iter.ptr.as_ptr(), level as u32) };
        ret != 0
    }

    /// Returns `true` if the element is at the end of the given level.
    pub fn is_at_final_element(&self, level: LayoutLevel, element: LayoutLevel) -> bool {
        let ret = unsafe {
            c::TessPageIteratorIsAtFinalElement(
                self.iter.ptr.as_ptr(),
                level as u32,
                element as u32,
            )
        };
        ret != 0
    }

    /// Returns the bounds of the element at the given level of the layout.
    ///
    /// Returns `None` if the given level doesn't exist.
    ///
    /// The box position and dimensions should match the ones of
    /// [`get_binary_image`](Self::get_binary_image)
    /// but may not match the ones of [`get_image`](Self::get_image) due to padding.
    pub fn bounding_box(&self, level: LayoutLevel) -> Option<Rectangle> {
        let mut left = 0;
        let mut top = 0;
        let mut right = 0;
        let mut bottom = 0;
        let ret = unsafe {
            c::TessPageIteratorBoundingBox(
                self.iter.ptr.as_ptr(),
                level as u32,
                &mut left,
                &mut top,
                &mut right,
                &mut bottom,
            )
        };
        (ret != 0).then_some(Rectangle {
            left: left as u32,
            top: top as u32,
            width: (right - left) as u32,
            height: (bottom - top) as u32,
        })
    }

    /// Returns block type.
    pub fn block_type(&self) -> BlockType {
        let ret = unsafe { c::TessPageIteratorBlockType(self.iter.ptr.as_ptr()) };
        BlockType::from_raw(ret)
    }

    /// Returns a binary image of the element at the given layout level.
    pub fn get_binary_image(&self, level: LayoutLevel) -> Image {
        let ptr =
            unsafe { c::TessPageIteratorGetBinaryImage(self.iter.ptr.as_ptr(), level as u32) };
        let ptr = NonNull::new(ptr).expect("TessPageIteratorGetBinaryImage returned NULL");
        Image { ptr }
    }

    /// Returns a grayscale image of the element at the given layout level.
    ///
    /// Padding is added to the left-top corner in both dimensions.
    ///
    /// If `padding` is zero, then the returned positions and dimensions should match
    /// the ones of [`bounding_box`](Self::bounding_box).
    pub fn get_image(
        &self,
        level: LayoutLevel,
        padding: u32,
        original: &Image,
    ) -> (Image, i32, i32) {
        let mut left: i32 = 0;
        let mut top: i32 = 0;
        let ptr = unsafe {
            c::TessPageIteratorGetImage(
                self.iter.ptr.as_ptr(),
                level as u32,
                padding as i32,
                original.ptr.as_ptr(),
                &mut left,
                &mut top,
            )
        };
        let ptr = NonNull::new(ptr).expect("TessPageIteratorGetImage returned NULL");
        (Image { ptr }, left, top)
    }

    /// Returns the baseline of the element.
    pub fn baseline(&self, level: LayoutLevel) -> Option<Line> {
        let mut x1 = 0;
        let mut y1 = 0;
        let mut x2 = 0;
        let mut y2 = 0;
        let ret = unsafe {
            c::TessPageIteratorBaseline(
                self.iter.ptr.as_ptr(),
                level as u32,
                &mut x1,
                &mut y1,
                &mut x2,
                &mut y2,
            )
        };
        (ret != 0).then_some(Line {
            x1: x1 as u32,
            y1: y1 as u32,
            x2: x2 as u32,
            y2: y2 as u32,
        })
    }

    /// Returns the element's block orientation.
    pub fn orientation(&self) -> OrientationParams {
        let mut orientation = 0;
        let mut writing_direction = 0;
        let mut textline_order = 0;
        let mut deskew_angle: f32 = 0.0;
        unsafe {
            c::TessPageIteratorOrientation(
                self.iter.ptr.as_ptr(),
                &mut orientation,
                &mut writing_direction,
                &mut textline_order,
                &mut deskew_angle,
            )
        };
        OrientationParams {
            orientation: Orientation::from_raw(orientation),
            writing_direction: WritingDirection::from_raw(writing_direction),
            textline_order: TextlineOrder::from_raw(textline_order),
            deskew_angle,
        }
    }

    /// Returns the element's paragraph information.
    pub fn paragraph_info(&self) -> ParagraphInfo {
        let mut justification = 0;
        let mut is_list_item = 0;
        let mut is_crown = 0;
        let mut first_line_indent = 0;
        unsafe {
            c::TessPageIteratorParagraphInfo(
                self.iter.ptr.as_ptr(),
                &mut justification,
                &mut is_list_item,
                &mut is_crown,
                &mut first_line_indent,
            )
        };
        ParagraphInfo {
            justification: ParagraphJustification::from_raw(justification),
            is_list_item: is_list_item != 0,
            is_crown: is_crown != 0,
            first_line_indent,
        }
    }
}
