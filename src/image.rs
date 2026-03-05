use core::ptr::NonNull;

use crate::InvalidImage;

fn rgb_to_rgba([r, g, b]: [u8; 3]) -> u32 {
    u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16)
}

fn rgba_to_rgba([r, g, b, a]: [u8; 4]) -> u32 {
    u32::from(r) | (u32::from(g) << 8) | (u32::from(b) << 16) | (u32::from(a) << 24)
}

/// Tesseract-specific image.
pub struct Image {
    pub(crate) ptr: NonNull<c::PIX>,
}

impl Image {
    /// Create an image from raw RGB bytes.
    pub fn from_rgb(width: u32, height: u32, rgb: &[u8]) -> Result<Self, InvalidImage> {
        assert!(
            width <= i32::MAX as u32
                && height <= i32::MAX as u32
                && u64::from(width) * u64::from(height) * 3 == rgb.len() as u64
        );
        let bits_per_pixel = 32;
        let mut image = Self::new(width, height, bits_per_pixel)?;
        let pixels = image.as_pixels_mut();
        for (pixel, rgb) in pixels.iter_mut().zip(rgb.chunks_exact(3)) {
            *pixel = rgb_to_rgba([rgb[0], rgb[1], rgb[2]]);
        }
        Ok(image)
    }

    /// Create an image from raw RGBA bytes.
    pub fn from_rgba(width: u32, height: u32, rgba: &[u8]) -> Result<Self, InvalidImage> {
        assert!(
            width <= i32::MAX as u32
                && height <= i32::MAX as u32
                && u64::from(width) * u64::from(height) * 4 == rgba.len() as u64
        );
        let bits_per_pixel = 32;
        let mut image = Self::new(width, height, bits_per_pixel)?;
        let pixels = image.as_pixels_mut();
        for (pixel, rgba) in pixels.iter_mut().zip(rgba.chunks_exact(4)) {
            *pixel = rgba_to_rgba([rgba[0], rgba[1], rgba[2], rgba[3]]);
        }
        Ok(image)
    }

    fn new(width: u32, height: u32, bits_per_pixel: u32) -> Result<Self, InvalidImage> {
        let ptr = unsafe { c::pixCreate(width as i32, height as i32, bits_per_pixel as i32) };
        let ptr = NonNull::new(ptr).ok_or(InvalidImage)?;
        Ok(Self { ptr })
    }

    fn as_pixels_mut(&mut self) -> &mut [u32] {
        let (ptr, len) = self.get_raw_pixels();
        unsafe { core::slice::from_raw_parts_mut(ptr.cast(), len) }
    }

    fn get_raw_pixels(&self) -> (*mut u32, usize) {
        let data_ptr = unsafe { c::pixGetData(self.ptr.as_ptr()) };
        let wpl = unsafe { c::pixGetWpl(self.ptr.as_ptr()) };
        let width = unsafe { c::pixGetWidth(self.ptr.as_ptr()) };
        let height = unsafe { c::pixGetHeight(self.ptr.as_ptr()) };
        assert!(wpl >= 0 && height >= 0 && width == wpl && !data_ptr.is_null());
        let len: usize = (wpl as usize)
            .checked_mul(height as usize)
            .expect("Overflow");
        (data_ptr, len)
    }

    /// Returns width and height of the image.
    pub fn dimensions(&self) -> (u32, u32) {
        let mut width = 0;
        let mut height = 0;
        let mut depth = 0;
        let ret =
            unsafe { c::pixGetDimensions(self.ptr.as_ptr(), &mut width, &mut height, &mut depth) };
        assert!(ret == 0);
        let _ = depth;
        (width as u32, height as u32)
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe { c::pixDestroy(&mut self.ptr.as_ptr()) };
    }
}

impl Clone for Image {
    fn clone(&self) -> Self {
        let ptr = unsafe { c::pixClone(self.ptr.as_ptr()) };
        let ptr = NonNull::new(ptr).expect("pixClone returned NULL");
        Self { ptr }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LayoutLevel;
    use crate::TextRecognizer;

    #[test]
    fn ocr_test_image() {
        let text = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/data/text.txt"));
        let rgb = image::ImageReader::open(concat!(env!("CARGO_MANIFEST_DIR"), "/data/text.png"))
            .unwrap()
            .decode()
            .unwrap()
            .into_rgb8();
        let image = Image::from_rgb(rgb.width(), rgb.height(), rgb.as_raw()).unwrap();
        let mut recognizer = TextRecognizer::new().unwrap();
        let results = recognizer.recognize_text(&image).unwrap();
        assert_eq!(text, results.get_utf8_text().as_str());
    }
}
