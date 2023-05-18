use std::fs::File;
use std::io;
use std::io::Read;
use std::path::Path;

use crate::text::SourceText;

pub fn read_source_text(
    path: &Path,
) -> Result<SourceText, io::Error> {
    let mut file = File::open(path)?;
    let mut text = String::new();
    file.read_to_string(&mut text)?;
    Ok(SourceText::new(text))
}