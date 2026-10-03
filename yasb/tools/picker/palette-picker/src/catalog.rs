//! The palette catalog: `palette-themes.json` in the picker folder.
//!
//! Hand-rolled JSON on purpose. The file is a flat array of objects with
//! only string fields and one string array, so a full parser dependency
//! would be more surface area than the format warrants — and this crate
//! already depends on `windows`, so keeping the dependency list short
//! matters more than generality here. Unknown syntax is an error rather
//! than something silently skipped, so a corrupted catalog surfaces
//! instead of showing an empty picker.

use std::path::Path;

use yasb_theme::parse_color;

/// One selectable palette.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// Full theme name, exactly as `yasb-theme set` expects it.
    pub name: String,
    /// Three swatch colours as authored in the JSON.
    pub colors: Vec<String>,
    /// `"dark"` or `"light"`; drives the section heading.
    pub section: String,
}

impl Item {
    /// Swatches resolved to RGBA, dropping any the CSS colour parser
    /// rejects. A bad hex costs one chip rather than the whole row.
    pub fn swatches(&self) -> Vec<yasb_theme::Rgba> {
        self.colors.iter().filter_map(|c| parse_color(c)).collect()
    }
}

/// Read and parse the catalog, with a clear error on malformed input.
pub fn load(path: &Path) -> Result<Vec<Item>, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    parse(&raw)
}

/// Parse the catalog text. Exposed so tests need no files on disk.
pub fn parse(text: &str) -> Result<Vec<Item>, String> {
    let mut p = Parser { b: text.as_bytes(), i: 0 };
    p.ws();
    let items = p.items()?;
    p.ws();
    if p.i != p.b.len() {
        return Err(format!("trailing data at byte {}", p.i));
    }
    if items.is_empty() {
        return Err("catalog is empty".to_string());
    }
    Ok(items)
}

struct Parser<'a> {
    b: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && self.b[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }

    fn eat(&mut self, c: u8) -> Result<(), String> {
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(format!(
                "expected '{}' at byte {}, found {:?}",
                c as char,
                self.i,
                self.peek().map(|b| b as char)
            ))
        }
    }

    /// `"name": { ... }` entries inside the top-level array.
    fn items(&mut self) -> Result<Vec<Item>, String> {
        self.eat(b'[')?;
        let mut out = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(out);
        }
        loop {
            self.ws();
            out.push(self.item()?);
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(out);
                }
                other => return Err(format!("expected ',' or ']' at byte {}, found {other:?}", self.i)),
            }
        }
    }

    fn item(&mut self) -> Result<Item, String> {
        self.eat(b'{')?;
        let mut name = String::new();
        let mut colors = Vec::new();
        let mut section = String::new();
        loop {
            self.ws();
            let key = self.string()?;
            self.ws();
            self.eat(b':')?;
            self.ws();
            match key.as_str() {
                "name" => name = self.string()?,
                "colors" => colors = self.string_array()?,
                "section" => section = self.string()?,
                // Skip a value of an unrecognised type rather than failing:
                // a future field should not break today's picker.
                _ => self.skip_value()?,
            }
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    break;
                }
                other => return Err(format!("expected ',' or '}}' at byte {}, found {other:?}", self.i)),
            }
        }
        if name.is_empty() {
            return Err(format!("catalog entry at byte {} has no name", self.i));
        }
        Ok(Item { name, colors, section })
    }

    fn string(&mut self) -> Result<String, String> {
        self.eat(b'"')?;
        let mut s = String::new();
        loop {
            let Some(c) = self.peek() else {
                return Err("unterminated string".to_string());
            };
            self.i += 1;
            match c {
                b'"' => return Ok(s),
                b'\\' => {
                    let Some(e) = self.peek() else {
                        return Err("unterminated escape".to_string());
                    };
                    self.i += 1;
                    match e {
                        b'n' => s.push('\n'),
                        b't' => s.push('\t'),
                        b'r' => s.push('\r'),
                        b'b' => s.push('\u{8}'),
                        b'f' => s.push('\u{c}'),
                        b'u' => s.push(self.unicode_escape()?),
                        other => s.push(other as char),
                    }
                }
                _ => {
                    // Re-assemble the UTF-8 sequence for non-ASCII bytes.
                    let start = self.i - 1;
                    let len = utf8_len(c);
                    self.i = start + len;
                    let chunk = self
                        .b
                        .get(start..self.i)
                        .ok_or_else(|| "truncated UTF-8 in string".to_string())?;
                    s.push_str(std::str::from_utf8(chunk).map_err(|e| e.to_string())?);
                }
            }
        }
    }

    fn unicode_escape(&mut self) -> Result<char, String> {
        let hex = self
            .b
            .get(self.i..self.i + 4)
            .ok_or_else(|| "short \\u escape".to_string())?;
        self.i += 4;
        let n = u32::from_str_radix(std::str::from_utf8(hex).map_err(|e| e.to_string())?, 16)
            .map_err(|e| e.to_string())?;
        char::from_u32(n).ok_or_else(|| format!("bad code point U+{n:04X}"))
    }

    fn string_array(&mut self) -> Result<Vec<String>, String> {
        self.eat(b'[')?;
        let mut out = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(out);
        }
        loop {
            self.ws();
            out.push(self.string()?);
            self.ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {
                    self.i += 1;
                    return Ok(out);
                }
                other => return Err(format!("expected ',' or ']' at byte {}, found {other:?}", self.i)),
            }
        }
    }

    fn skip_value(&mut self) -> Result<(), String> {
        self.ws();
        match self.peek() {
            Some(b'"') => {
                self.string()?;
            }
            Some(b'{') | Some(b'[') => {
                let mut depth = 0usize;
                while let Some(c) = self.peek() {
                    match c {
                        b'"' => {
                            // Consume the whole string without counting its
                            // braces: "}" inside a string is not a delimiter.
                            self.string()?;
                            continue;
                        }
                        b'{' | b'[' => depth += 1,
                        b'}' | b']' => {
                            depth -= 1;
                            if depth == 0 {
                                self.i += 1;
                                return Ok(());
                            }
                        }
                        _ => {}
                    }
                    self.i += 1;
                }
                return Err("unterminated container".to_string());
            }
            _ => {
                while let Some(c) = self.peek() {
                    if c == b',' || c == b'}' || c == b']' || c.is_ascii_whitespace() {
                        break;
                    }
                    self.i += 1;
                }
            }
        }
        Ok(())
    }
}

/// Bytes in a UTF-8 sequence starting with `first`.
fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yasb_theme::Rgba;

    // r## because the hex literals contain `"#`, which would close an r#"..."#.
    const SAMPLE: &str = r##"[
      { "name": "Rangalipi", "colors": ["#d99a2b", "#6fc3c9", "#e07a4f"], "section": "dark" },
      { "name": "Rangalipi Wine", "colors": ["#c04e68"], "section": "dark" },
      { "name": "Rangalipi Light", "colors": ["#b37a1a"], "section": "light" }
    ]"##;

    #[test]
    fn reads_every_field() {
        let items = parse(SAMPLE).unwrap();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].name, "Rangalipi");
        assert_eq!(items[0].colors.len(), 3);
        assert_eq!(items[2].section, "light");
    }

    #[test]
    fn resolves_swatch_colours() {
        let items = parse(SAMPLE).unwrap();
        assert_eq!(
            items[1].swatches(),
            vec![Rgba::rgb(0xC0, 0x4E, 0x68)]
        );
    }

    #[test]
    fn tolerates_whitespace_and_unknown_fields() {
        let src = r##"[ { "name":"A", "future": {"x":[1,2]}, "colors":["#fff"], "section":"dark" } ]"##;
        let items = parse(src).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "A");
    }

    #[test]
    fn rejects_malformed_input() {
        assert!(parse("").is_err());
        assert!(parse("[]").is_err(), "empty catalog must be an error");
        assert!(parse("[{ \"name\": }]").is_err());
        assert!(parse(r#"[{"name":"A","colors":[]}] trailing"#).is_err());
    }

    #[test]
    fn real_catalog_has_all_themes() {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../palette-themes.json");
        let items = load(&p).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(items.len(), 22);
        for it in &items {
            assert!(!it.name.is_empty());
            assert!(
                !it.swatches().is_empty(),
                "{} has no parseable swatch",
                it.name
            );
        }
    }
}