#![doc = include_str!("../README.md")]

mod c;
mod common;
mod error;
mod image;
mod layout;
mod ocr;
mod types;
mod vars;
mod version;

pub use self::common::*;
pub use self::error::*;
pub use self::image::*;
pub use self::layout::*;
pub use self::ocr::*;
pub use self::types::*;
pub use self::version::*;

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
