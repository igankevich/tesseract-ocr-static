use core::ffi::CStr;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;

use crate::InvalidVariable;
use crate::Tesseract;
use crate::WriteFailed;
use crate::c;

impl Tesseract {
    /// Set tesseract variable.
    ///
    /// # How to improve text recognition?
    ///
    /// There is a guide on how to improve text recognition:
    /// <https://tesseract-ocr.github.io/tessdoc/ImproveQuality.html>
    ///
    /// # Variables
    ///
    #[doc = include_str!("../variables.md")]
    pub fn set_variable(&mut self, name: &CStr, value: &CStr) -> Result<(), InvalidVariable> {
        let ret =
            unsafe { c::TessBaseAPISetVariable(self.ptr.as_ptr(), name.as_ptr(), value.as_ptr()) };
        if ret != 0 {
            Err(InvalidVariable)
        } else {
            Ok(())
        }
    }

    /// Set tesseract variable.
    ///
    /// Includes debug variables.
    ///
    /// See [set_variable](Self::set_variable) for more information.
    pub fn set_debug_variable(&mut self, name: &CStr, value: &CStr) -> Result<(), InvalidVariable> {
        let ret = unsafe {
            c::TessBaseAPISetDebugVariable(self.ptr.as_ptr(), name.as_ptr(), value.as_ptr())
        };
        if ret != 0 {
            Err(InvalidVariable)
        } else {
            Ok(())
        }
    }

    /// Get integer variable value.
    pub fn get_variable_i32(&self, name: &CStr) -> Option<i32> {
        let mut value = 0;
        let ret =
            unsafe { c::TessBaseAPIGetIntVariable(self.ptr.as_ptr(), name.as_ptr(), &mut value) };
        (ret != 0).then_some(value)
    }

    /// Get boolean variable value.
    pub fn get_variable_bool(&self, name: &CStr) -> Option<bool> {
        let mut value = 0;
        let ret =
            unsafe { c::TessBaseAPIGetBoolVariable(self.ptr.as_ptr(), name.as_ptr(), &mut value) };
        (ret != 0).then_some(value != 0)
    }

    /// Get floating point variable value.
    pub fn get_variable_f64(&self, name: &CStr) -> Option<f64> {
        let mut value = 0.0;
        let ret = unsafe {
            c::TessBaseAPIGetDoubleVariable(self.ptr.as_ptr(), name.as_ptr(), &mut value)
        };
        (ret != 0).then_some(value)
    }

    /// Get string variable value.
    pub fn get_variable_c_str<'a>(&'a self, name: &CStr) -> Option<&'a CStr> {
        let ptr = unsafe { c::TessBaseAPIGetStringVariable(self.ptr.as_ptr(), name.as_ptr()) };
        if ptr.is_null() {
            return None;
        }
        Some(unsafe { CStr::from_ptr(ptr) })
    }

    /// Read variables from the configuration file.
    ///
    /// Doesn't include debug variables.
    ///
    /// This is a re-implementation of the original library call that handles I/O errors and
    /// invalid variables.
    pub fn read_config_file(&mut self, filename: &Path) -> std::io::Result<()> {
        do_read_config_file(filename, |name, value| self.set_variable(name, value))
    }

    /// Read variables from the configuration file.
    ///
    /// Includes debug variables.
    ///
    /// This is a re-implementation of the original library call that handles I/O errors and
    /// invalid variables.
    pub fn read_debug_config_file(&mut self, filename: &Path) -> std::io::Result<()> {
        do_read_config_file(filename, |name, value| self.set_debug_variable(name, value))
    }

    /// Print all variables to a file.
    pub fn print_variables_to_file(&self, filename: &CStr) -> Result<(), WriteFailed> {
        let ret =
            unsafe { c::TessBaseAPIPrintVariablesToFile(self.ptr.as_ptr(), filename.as_ptr()) };
        if ret < 0 { Err(WriteFailed) } else { Ok(()) }
    }
}

fn do_read_config_file(
    filename: &Path,
    mut set_variable: impl FnMut(&CStr, &CStr) -> Result<(), InvalidVariable>,
) -> std::io::Result<()> {
    use std::io::ErrorKind::InvalidData;
    let file = BufReader::new(File::open(filename)?);
    for line in file.lines() {
        let mut line = line?.into_bytes();
        // Add NUL byte.
        line.push(0);
        let mut line = line.as_mut_slice();
        // Trim whitespace from the left.
        while let Some(ch) = line.first() {
            if !ch.is_ascii_whitespace() {
                break;
            }
            line = &mut line[1..];
        }
        // Skip comments and empty lines.
        match line.first() {
            Some(&b'#') | None => continue,
            _ => {}
        }
        let i = line
            .iter()
            .position(|ch| ch.is_ascii_whitespace())
            .ok_or(InvalidData)?;
        // Insert NUL byte.
        line[i] = 0;
        let name = &line[..i];
        let mut value = &line[i + 1..];
        // Trim whitespace from the left.
        while let Some(ch) = value.first() {
            if !ch.is_ascii_whitespace() {
                break;
            }
            value = &value[1..];
        }
        let name = CStr::from_bytes_with_nul(name).map_err(|_| InvalidData)?;
        let value = CStr::from_bytes_with_nul(value).map_err(|_| InvalidData)?;
        set_variable(name, value).map_err(std::io::Error::other)?;
    }
    Ok(())
}
