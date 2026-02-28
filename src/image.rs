use core::mem::size_of;
use core::mem::size_of_val;
use core::ptr::NonNull;

use crate::c;
use crate::InvalidImage;

pub struct Image {
    pub(crate) ptr: NonNull<c::PIX>,
}

impl Image {
    pub fn new(width: u32, height: u32, bits_per_pixel: u32) -> Result<Self, InvalidImage> {
        let ptr = unsafe { c::pixCreate(width as i32, height as i32, bits_per_pixel as i32) };
        let ptr = NonNull::new(ptr).ok_or(InvalidImage)?;
        Ok(Self { ptr })
    }

    pub fn read_mem(bytes: &[u8]) -> Result<Self, InvalidImage> {
        let ptr = unsafe { c::pixReadMem(bytes.as_ptr(), size_of_val(bytes)) };
        let ptr = NonNull::new(ptr).ok_or(InvalidImage)?;
        Ok(Self { ptr })
    }

    pub fn as_pixels(&self) -> &[u32] {
        let (ptr, len) = self.get_raw_pixels();
        unsafe { core::slice::from_raw_parts(ptr.cast(), len) }
    }

    pub fn as_pixels_mut(&mut self) -> &mut [u32] {
        let (ptr, len) = self.get_raw_pixels();
        unsafe { core::slice::from_raw_parts_mut(ptr.cast(), len) }
    }

    fn get_raw_pixels(&self) -> (*mut u32, usize) {
        let data_ptr = unsafe { c::pixGetData(self.ptr.as_ptr()) };
        let wpl = unsafe { c::pixGetWpl(self.ptr.as_ptr()) };
        let height = unsafe { c::pixGetHeight(self.ptr.as_ptr()) };
        assert!(wpl >= 0 && height >= 0 && !data_ptr.is_null());
        let len: usize = (wpl as usize)
            .checked_mul(height as usize)
            .and_then(|words| words.checked_mul(size_of::<u32>()))
            .expect("Overflow");
        (data_ptr, len)
    }

    pub fn dimensions(&self) -> (u32, u32, u32) {
        let mut width = 0;
        let mut height = 0;
        let mut depth = 0;
        let ret =
            unsafe { c::pixGetDimensions(self.ptr.as_ptr(), &mut width, &mut height, &mut depth) };
        assert!(ret == 0);
        (width as u32, height as u32, depth as u32)
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
