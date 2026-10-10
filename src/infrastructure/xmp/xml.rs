//! Bounded XML tree for XMP. No DTD, entity expansion or external resources.
//! Names/namespace declarations are retained, including unknown vendor nodes.
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Node {
    Element(Element),
    Text(String),
    Raw(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Element {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Node>,
}

pub(super) type Namespaces = BTreeMap<String, String>;

impl Element {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.into(),
            attributes: Vec::new(),
            children: Vec::new(),
        }
    }
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
    pub fn set_attr(&mut self, key: &str, value: &str) {
        self.attributes.retain(|(name, _)| name != key);
        self.attributes.push((key.into(), value.into()));
    }
    pub fn namespaces(&self, inherited: &Namespaces) -> Namespaces {
        let mut result = inherited.clone();
        result.insert("xml".into(), "http://www.w3.org/XML/1998/namespace".into());
        for (name, value) in &self.attributes {
            if name == "xmlns" {
                result.insert(String::new(), value.clone());
            } else if let Some(prefix) = name.strip_prefix("xmlns:") {
                result.insert(prefix.into(), value.clone());
            }
        }
        result
    }
    pub fn text(&self) -> String {
        self.children
            .iter()
            .filter_map(|node| match node {
                Node::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect()
    }
}

pub(super) fn expanded<'a>(
    name: &'a str,
    ns: &'a Namespaces,
    attribute: bool,
) -> Option<(&'a str, &'a str)> {
    if let Some((prefix, local)) = name.split_once(':') {
        Some((ns.get(prefix)?.as_str(), local))
    } else {
        Some((
            if attribute {
                ""
            } else {
                ns.get("").map(String::as_str).unwrap_or("")
            },
            name,
        ))
    }
}

pub(super) fn parse(bytes: &[u8]) -> Result<Element, String> {
    let text = decode(bytes)?.replace("\r\n", "\n").replace('\r', "\n");
    if text.chars().any(|c| !valid_char(c)) {
        return Err("Invalid XML character".into());
    }
    let mut parser = Parser {
        text: &text,
        pos: 0,
        nodes: 0,
    };
    parser.misc()?;
    let root = parser.element(0)?;
    parser.misc()?;
    if parser.pos != text.len() {
        return Err("Content after XML root".into());
    }
    validate_namespaces(&root, &Namespaces::new())?;
    Ok(root)
}

fn decode(bytes: &[u8]) -> Result<String, String> {
    let (width, little, skip) = if bytes.starts_with(&[0xff, 0xfe, 0, 0]) {
        (4, true, 4)
    } else if bytes.starts_with(&[0, 0, 0xfe, 0xff]) {
        (4, false, 4)
    } else if bytes.starts_with(&[0xff, 0xfe]) {
        (2, true, 2)
    } else if bytes.starts_with(&[0xfe, 0xff]) {
        (2, false, 2)
    } else if bytes.starts_with(&[0, 0, 0, b'<']) {
        (4, false, 0)
    } else if bytes.starts_with(&[b'<', 0, 0, 0]) {
        (4, true, 0)
    } else if bytes.starts_with(&[0, b'<', 0]) {
        (2, false, 0)
    } else if bytes.starts_with(&[b'<', 0]) {
        (2, true, 0)
    } else {
        return std::str::from_utf8(bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes))
            .map(str::to_owned)
            .map_err(|_| "Invalid XMP UTF-8".into());
    };
    let bytes = &bytes[skip..];
    if bytes.len() % width != 0 {
        return Err("Truncated XMP Unicode encoding".into());
    }
    if width == 2 {
        char::decode_utf16(bytes.chunks_exact(2).map(|b| {
            if little {
                u16::from_le_bytes([b[0], b[1]])
            } else {
                u16::from_be_bytes([b[0], b[1]])
            }
        }))
        .map(|c| c.map_err(|_| "Invalid XMP UTF-16".to_owned()))
        .collect()
    } else {
        bytes
            .chunks_exact(4)
            .map(|b| {
                let b = [b[0], b[1], b[2], b[3]];
                char::from_u32(if little {
                    u32::from_le_bytes(b)
                } else {
                    u32::from_be_bytes(b)
                })
                .ok_or("Invalid XMP UTF-32".to_owned())
            })
            .collect()
    }
}

fn valid_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r')
        || matches!(c as u32, 0x20..=0xd7ff | 0xe000..=0xfffd | 0x10000..=0x10ffff)
}
fn name_start(c: char) -> bool {
    c == ':'
        || c == '_'
        || c.is_ascii_alphabetic()
        || matches!(c as u32,
        0xc0..=0xd6 | 0xd8..=0xf6 | 0xf8..=0x2ff | 0x370..=0x37d | 0x37f..=0x1fff |
        0x200c..=0x200d | 0x2070..=0x218f | 0x2c00..=0x2fef | 0x3001..=0xd7ff |
        0xf900..=0xfdcf | 0xfdf0..=0xfffd | 0x10000..=0xeffff)
}

struct Parser<'a> {
    text: &'a str,
    pos: usize,
    nodes: usize,
}
impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }
    fn space(&mut self) {
        while self.rest().starts_with([' ', '\t', '\n', '\r']) {
            self.pos += 1;
        }
    }
    fn expect(&mut self, token: &str) -> Result<(), String> {
        if self.rest().starts_with(token) {
            self.pos += token.len();
            Ok(())
        } else {
            Err(format!("Expected XML {token}"))
        }
    }
    fn terminated(&mut self, end: &str) -> Result<String, String> {
        let count = self.rest().find(end).ok_or("Unterminated XML token")?;
        let value = self.rest()[..count].to_owned();
        self.pos += count + end.len();
        Ok(value)
    }
    fn misc(&mut self) -> Result<(), String> {
        loop {
            self.space();
            if self.rest().starts_with("<?") {
                self.pos += 2;
                self.terminated("?>")?;
            } else if self.rest().starts_with("<!--") {
                self.pos += 4;
                let comment = self.terminated("-->")?;
                if comment.contains("--") {
                    return Err("Invalid XML comment".into());
                }
            } else {
                break;
            }
        }
        Ok(())
    }
    fn name(&mut self) -> Result<String, String> {
        let start = self.pos;
        for (index, c) in self.text[start..].char_indices() {
            let valid = name_start(c)
                || (index > 0
                    && (c == '-'
                        || c == '.'
                        || c.is_ascii_digit()
                        || matches!(c as u32,0xb7 | 0x300..=0x36f | 0x203f..=0x2040)));
            if !valid {
                break;
            }
            self.pos = start + index + c.len_utf8();
        }
        if self.pos == start {
            return Err("Invalid XML name".into());
        }
        let name = &self.text[start..self.pos];
        if name.matches(':').count() > 1 || name.starts_with(':') || name.ends_with(':') {
            return Err("Invalid XML QName".into());
        }
        Ok(name.into())
    }
    fn element(&mut self, depth: usize) -> Result<Element, String> {
        self.nodes += 1;
        if depth > 96 || self.nodes > 200_000 {
            return Err("XMP XML complexity limit exceeded".into());
        }
        self.expect("<")?;
        let mut node = Element::new(&self.name()?);
        loop {
            let before = self.pos;
            self.space();
            if self.rest().starts_with("/>") {
                self.pos += 2;
                return Ok(node);
            }
            if self.rest().starts_with('>') {
                self.pos += 1;
                break;
            }
            if self.pos == before {
                return Err("Missing XML attribute separator".into());
            }
            let key = self.name()?;
            self.space();
            self.expect("=")?;
            self.space();
            let quote = self.rest().chars().next().ok_or("Missing XML quote")?;
            if quote != '\'' && quote != '"' {
                return Err("Unquoted XML attribute".into());
            }
            self.pos += 1;
            let count = self
                .rest()
                .find(quote)
                .ok_or("Unterminated XML attribute")?;
            let raw = &self.rest()[..count];
            if raw.contains('<') {
                return Err("Invalid XML attribute".into());
            }
            let value = unescape(&raw.replace(['\t', '\n', '\r'], " "))?;
            self.pos += count + 1;
            if node.attr(&key).is_some() {
                return Err("Duplicate XML attribute".into());
            }
            node.attributes.push((key, value));
        }
        loop {
            if self.rest().starts_with("</") {
                self.pos += 2;
                if self.name()? != node.name {
                    return Err("Mismatched XML element".into());
                }
                self.space();
                self.expect(">")?;
                return Ok(node);
            }
            if self.rest().starts_with("<![CDATA[") {
                self.pos += 9;
                node.children.push(Node::Text(self.terminated("]]>")?));
            } else if self.rest().starts_with("<!--") {
                self.pos += 4;
                let comment = self.terminated("-->")?;
                if comment.contains("--") {
                    return Err("Invalid XML comment".into());
                }
                node.children.push(Node::Raw(format!("<!--{comment}-->")));
            } else if self.rest().starts_with("<?") {
                self.pos += 2;
                node.children
                    .push(Node::Raw(format!("<?{}?>", self.terminated("?>")?)));
            } else if self.rest().starts_with("<!") {
                return Err("XMP DTD/declarations are not supported".into());
            } else if self.rest().starts_with('<') {
                node.children.push(Node::Element(self.element(depth + 1)?));
            } else {
                let count = self.rest().find('<').ok_or("Unclosed XML element")?;
                let raw = &self.rest()[..count];
                if raw.contains("]]>") {
                    return Err("Invalid XML text".into());
                }
                node.children.push(Node::Text(unescape(raw)?));
                self.pos += count;
            }
        }
    }
}

fn unescape(raw: &str) -> Result<String, String> {
    let mut result = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find('&') {
        result.push_str(&rest[..start]);
        rest = &rest[start + 1..];
        let end = rest.find(';').ok_or("Unterminated XML entity")?;
        let entity = &rest[..end];
        let c = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ => {
                let value = if let Some(hex) = entity.strip_prefix("#x") {
                    u32::from_str_radix(hex, 16).ok()
                } else if let Some(decimal) = entity.strip_prefix('#') {
                    decimal.parse().ok()
                } else {
                    None
                };
                value
                    .and_then(char::from_u32)
                    .filter(|c| valid_char(*c))
                    .ok_or("Unsupported/invalid XML entity")?
            }
        };
        result.push(c);
        rest = &rest[end + 1..];
    }
    result.push_str(rest);
    Ok(result)
}

fn validate_namespaces(node: &Element, inherited: &Namespaces) -> Result<(), String> {
    for (key, value) in &node.attributes {
        if key == "xmlns" || key.starts_with("xmlns:") {
            let prefix = key.strip_prefix("xmlns:").unwrap_or("");
            if prefix == "xmlns"
                || value == "http://www.w3.org/2000/xmlns/"
                || (prefix == "xml") != (value == "http://www.w3.org/XML/1998/namespace")
                || (!prefix.is_empty() && value.is_empty())
            {
                return Err("Invalid XML namespace binding".into());
            }
        }
    }
    let ns = node.namespaces(inherited);
    if expanded(&node.name, &ns, false).is_none() {
        return Err("Undeclared XMP namespace".into());
    }
    let mut attributes = std::collections::BTreeSet::new();
    for (key, _) in &node.attributes {
        if key == "xmlns" || key.starts_with("xmlns:") {
            continue;
        }
        let name = expanded(key, &ns, true).ok_or("Undeclared attribute namespace")?;
        if !attributes.insert(name) {
            return Err("Duplicate expanded XML attribute".into());
        }
    }
    for child in &node.children {
        if let Node::Element(child) = child {
            validate_namespaces(child, &ns)?;
        }
    }
    Ok(())
}

pub(super) fn serialize(node: &Element) -> Result<Vec<u8>, String> {
    let mut output = String::new();
    write_element(node, &mut output)?;
    Ok(output.into_bytes())
}

fn escaped(text: &str, attribute: bool, out: &mut String) -> Result<(), String> {
    for c in text.chars() {
        if !valid_char(c) {
            return Err("Invalid XMP property character".into());
        }
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            '\r' => out.push_str("&#xD;"),
            '\n' if attribute => out.push_str("&#xA;"),
            '\t' if attribute => out.push_str("&#x9;"),
            _ => out.push(c),
        }
    }
    Ok(())
}

fn write_element(node: &Element, output: &mut String) -> Result<(), String> {
    output.push('<');
    output.push_str(&node.name);
    for (key, value) in &node.attributes {
        output.push(' ');
        output.push_str(key);
        output.push_str("=\"");
        escaped(value, true, output)?;
        output.push('"');
    }
    if node.children.is_empty() {
        output.push_str("/>");
        return Ok(());
    }
    output.push('>');
    for child in &node.children {
        match child {
            Node::Element(node) => write_element(node, output)?,
            Node::Text(text) => escaped(text, false, output)?,
            Node::Raw(raw) => output.push_str(raw),
        }
    }
    output.push_str("</");
    output.push_str(&node.name);
    output.push('>');
    Ok(())
}
