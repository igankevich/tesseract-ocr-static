macro_rules! errors {
    ($(($error: ident $description: literal))+) => {
        $(
            #[doc = $description]
            #[derive(Debug)]
            pub struct $error;

            impl std::error::Error for $error {}

            impl core::fmt::Display for $error {
                fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                    f.write_str($description)
                }
            }
        )+
    };
}

errors! {
    (InvalidImage "Invalid image")
    (InvalidVariable "Invalid variable")
    (RecognitionFailed "Recognition failed")
    (InitFailed "Initialization failed")
    (WriteFailed "Write failed")
}
