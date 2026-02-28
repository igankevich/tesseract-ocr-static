use std::ffi::CString;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;

use anyhow::anyhow;
use rustyract::TextRecognizer;
use tempfile::TempDir;

struct EscapeMarkdown<'a>(&'a str);

impl core::fmt::Display for EscapeMarkdown<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        for ch in self.0.chars() {
            match ch {
                '_' | '[' | ']' | '|' | '`' => write!(f, "\\{ch}")?,
                _ => write!(f, "{ch}")?,
            }
        }
        Ok(())
    }
}

struct EscapeMarkdownCode<'a>(&'a str);

impl core::fmt::Display for EscapeMarkdownCode<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        for ch in self.0.chars() {
            match ch {
                '|' => write!(f, "\\{ch}")?,
                _ => write!(f, "{ch}")?,
            }
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let tmp_dir = TempDir::new()?;
    let tmp_output_file = tmp_dir.path().join("vars");
    let tess = TextRecognizer::new()?;
    tess.print_variables_to_file(&CString::new(
        tmp_output_file.clone().into_os_string().into_encoded_bytes(),
    )?)?;
    let reader = BufReader::new(fs::File::open(&tmp_output_file)?);
    let mut lines: Vec<_> = reader.lines().map(|line| line.unwrap()).collect();
    lines.sort_unstable();
    let mut out_file = BufWriter::new(fs::File::create(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../variables.md"
    ))?);
    writeln!(&mut out_file, "| Variable | Default value | Description |")?;
    writeln!(&mut out_file, "|----------|---------------|------------ |")?;
    for line in lines.into_iter() {
        let mut columns = line.trim().splitn(3, '\t');
        let name = columns
            .next()
            .ok_or_else(|| anyhow!("Invalid `print_variables_to_file` output"))?;
        let default_value = columns
            .next()
            .ok_or_else(|| anyhow!("Invalid `print_variables_to_file` output"))?;
        let doc = columns
            .next()
            .ok_or_else(|| anyhow!("Invalid `print_variables_to_file` output"))?;
        let default_value_str =
            if default_value.parse::<i64>().is_ok() || default_value.parse::<f64>().is_ok() {
                format!("{default_value}")
            } else {
                format!("{default_value:?}")
            };
        writeln!(
            &mut out_file,
            "| `{name}` | ``{}`` | {} |",
            EscapeMarkdownCode(&default_value_str),
            EscapeMarkdown(doc)
        )?;
    }
    Ok(())
}
