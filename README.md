# tesseract-ocr-static

This crate provides ergonomic Rust interface for underlying [Tesseract OCR library](https://tesseract-ocr.github.io/).
There are two main structs: [`TextRecognizer`](crate::TextRecognizer) and [`LayoutAnalyzer`](crate::LayoutAnalyzer).
`TextRecognizer` allows one to recognize text from the picture and outputs the text, the bounding boxes and other parameters.
`LayoutAnalyzer` allows one to analyze the layout without recognizing text; it should consume less memory than `TextRecognizer`.

Pictures can be loaded into the library via [`Image`](crate::Image) struct that accepts images in raw RGB/RGBA formats; no other formats are supported.
If you need to read an image from a file in another format, you can do so with any Rust crate (e.g. [image](https://docs.rs/image/latest/image/)).
