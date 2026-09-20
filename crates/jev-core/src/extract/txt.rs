//! TXT/Markdown：按字节读入，UTF-8 lossy，剥离 BOM。

pub fn extract(bytes: &[u8]) -> String {
    let mut text = String::from_utf8_lossy(bytes).into_owned();
    if text.starts_with('\u{FEFF}') {
        text.remove(0);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_bom() {
        let bytes = [0xEF, 0xBB, 0xBF, b'h', b'i'];
        assert_eq!(extract(&bytes), "hi");
    }

    #[test]
    fn lossy_replacement_for_invalid_utf8() {
        let bytes = [0xFF, b'o', b'k'];
        assert_eq!(extract(&bytes), "\u{FFFD}ok");
    }
}
