This crate provides ergonomic Rust interface for underlying [Tesseract OCR library](https://tesseract-ocr.github.io/).
There are two main structs: [`TextRecognizer`](crate::TextRecognizer) and [`LayoutAnalyzer`](crate::LayoutAnalyzer).
`TextRecognizer` allows one to recognize text from the picture and outputs the text, the bounding boxes and other parameters.
`LayoutAnalyzer` allows one to analyze the layout without recognizing text; it should consume less memory than `TextRecognizer`.

Pictures can be loaded into the library via [`Image`](crate::Image) struct that accepts images in raw RGB/RGBA formats; no other formats are supported.
If you need to read an image from a file in another format, you can do so with any Rust crate (e.g. [image](https://docs.rs/image/latest/image/)).


## Examples

### Simple character recognition

```no_run
use tesseract_ocr_static::{Image, TextRecognizer};
use image::ImageReader;

let rgb = ImageReader::open("hello.txt").unwrap().decode().unwrap().into_rgb8();
let image = Image::from_rgb(rgb.width(), rgb.height(), rgb.as_raw()).unwrap();
let mut recognizer = TextRecognizer::new().unwrap();
let results = recognizer.recognize_text(&image).unwrap();
assert_eq!("Hello world", results.get_utf8_text().as_str());
```

### Print recognized text and corresponding bounding boxes

```no_run
use tesseract_ocr_static::{Image, LayoutLevel, TextRecognizer};
use image::ImageReader;

let rgb = ImageReader::open("hello.txt").unwrap().decode().unwrap().into_rgb8();
let image = Image::from_rgb(rgb.width(), rgb.height(), rgb.as_raw()).unwrap();
let mut recognizer = TextRecognizer::new().unwrap();
let results = recognizer.recognize_text(&image).unwrap();
let mut iter = results.iter();
while let Some(word) = iter.next(LayoutLevel::Word) {
    println!(
        "Word {:?}, confidence {:.1}, bounding box {:?}",
        word.get_utf8_text(LayoutLevel::Word).as_str(),
        word.confidence(LayoutLevel::Word),
        word.bounding_box(LayoutLevel::Word),
    );
}
```


## Build customization

Please consult [`tesseract-ocr-static-c`](https://docs.rs/tesseract-ocr-static-c) crate's documentation.
