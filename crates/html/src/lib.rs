//! HTML parser and DOM tree for NaBrowse.
//!
//! A from-scratch HTML tokenizer and tree builder. Produces an in-memory DOM
//! tree. Handles tags, attributes, text, comments, self-closing tags, and
//! basic error recovery for unclosed tags.
//!
//! Privacy: no external network calls, no telemetry. Pure local parsing.

use nabrowse_shared::NabrowseUrl;

/// A node in the DOM tree.
#[derive(Debug, Clone)]
pub enum Node {
    Element(Element),
    Text(Text),
    Comment(Comment),
    Document(Document),
}

/// An HTML element.
#[derive(Debug, Clone)]
pub struct Element {
    pub tag_name: String,
    pub attributes: Vec<(String, String)>,
    pub children: Vec<Node>,
    pub namespace: Option<String>,
}

/// Text node.
#[derive(Debug, Clone)]
pub struct Text {
    pub content: String,
}

/// Comment node.
#[derive(Debug, Clone)]
pub struct Comment {
    pub content: String,
}

/// The root document node.
#[derive(Debug, Clone)]
pub struct Document {
    pub children: Vec<Node>,
    pub url: Option<NabrowseUrl>,
    pub doctype: Option<String>,
}

impl Document {
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            url: None,
            doctype: None,
        }
    }
}

/// The HTML parser.
pub struct HtmlParser {
    input: String,
    position: usize,
}

impl HtmlParser {
    /// Create a new parser for the given HTML source.
    pub fn new(input: String) -> Self {
        Self { input, position: 0 }
    }

    /// Parse the entire input into a Document.
    pub fn parse(mut self) -> Document {
        let mut doc = Document::new();

        // Detect doctype.
        if self.input.trim_start().to_uppercase().starts_with("<!DOCTYPE") {
            doc.doctype = self.parse_doctype();
        }

        // Skip whitespace before root.
        self.skip_whitespace();

        // Parse root elements (typically a single <html>).
        while self.position < self.input.len() {
            let c = self.peek();
            if c == '<' {
                if let Some(node) = self.parse_node() {
                    doc.children.push(node);
                }
            } else if !c.is_whitespace() {
                // Text content at root level.
                if let Some(text) = self.parse_text_node() {
                    doc.children.push(Node::Text(text));
                }
            } else {
                self.advance();
            }
        }

        doc
    }

    fn parse_doctype(&mut self) -> Option<String> {
        // Consume "<!DOCTYPE"
        if !self.input[self.position..].to_uppercase().starts_with("<!DOCTYPE") {
            return None;
        }
        self.position += 9; // "<!DOCTYPE".len()

        // Find closing ">".
        let start = self.position;
        while self.position < self.input.len() {
            if self.input.as_bytes()[self.position] == b'>' {
                self.position += 1;
                let raw = &self.input[start..self.position - 1];
                return Some(raw.trim().to_string());
            }
            self.position += 1;
        }
        None
    }

    fn parse_node(&mut self) -> Option<Node> {
        // Expect '<'
        if self.peek() != '<' {
            return None;
        }
        self.advance(); // consume '<'

        // Closing tag?
        if self.peek() == '/' {
            self.advance();
            self.skip_until('>');
            self.advance(); // consume '>'
            return None;
        }

        // Comment?
        if self.input[self.position..].starts_with("!--") {
            self.position += 3;
            let start = self.position;
            while self.position < self.input.len() {
                if self.input[self.position..].starts_with("-->") {
                    self.position += 3;
                    let content = &self.input[start..self.position - 3];
                    return Some(Node::Comment(Comment {
                        content: content.to_string(),
                    }));
                }
                self.position += 1;
            }
            return None;
        }

        // Read tag name.
        let tag_start = self.position;
        while self.position < self.input.len() {
            let c = self.input.as_bytes()[self.position];
            if c.is_ascii_whitespace() || c == b'>' || c == b'/' || c == b'<' {
                break;
            }
            self.position += 1;
        }
        let tag_name = self.input[tag_start..self.position].to_string();

        // Read attributes.
        let mut attributes = Vec::new();
        let mut self_closing = false;

        loop {
            self.skip_whitespace_attr();
            if self.position >= self.input.len() {
                break;
            }
            let c = self.peek();
            if c == '>' {
                self.advance();
                break;
            }
            if c == '/' {
                self.advance();
                if self.peek() == '>' {
                    self.advance();
                }
                self_closing = true;
                break;
            }

            // Attribute name.
            let attr_start = self.position;
            while self.position < self.input.len() {
                let c = self.input.as_bytes()[self.position];
                if c.is_ascii_whitespace() || c == b'=' || c == b'>' || c == b'/' {
                    break;
                }
                self.position += 1;
            }
            let attr_name = self.input[attr_start..self.position].to_string();

            let mut attr_value = String::new();
            if self.peek() == '=' {
                self.advance();
                self.skip_whitespace_attr();
                let quote = self.peek();
                if quote == '"' || quote == '\'' {
                    self.advance();
                    let val_start = self.position;
                    while self.position < self.input.len() && self.peek() != quote {
                        self.position += 1;
                    }
                    attr_value = self.input[val_start..self.position].to_string();
                    if self.peek() == quote {
                        self.advance();
                    }
                } else {
                    // Unquoted value.
                    let val_start = self.position;
                    while self.position < self.input.len() {
                        let c = self.input.as_bytes()[self.position];
                        if c.is_ascii_whitespace() || c == b'>' || c == b'/' {
                            break;
                        }
                        self.position += 1;
                    }
                    attr_value = self.input[val_start..self.position].to_string();
                }
            }
            attributes.push((attr_name, attr_value));
        }

        // Void elements are self-closing.
        let is_void = matches!(
            tag_name.as_str(),
            "area" | "base" | "br" | "col" | "embed" | "hr" | "img" | "input"
                | "link" | "meta" | "param" | "source" | "track" | "wbr"
        );

        if self_closing || is_void {
            Some(Node::Element(Element {
                tag_name,
                attributes,
                children: Vec::new(),
                namespace: None,
            }))
        } else {
            // Parse children until closing tag.
            let mut children = Vec::new();
            let closing = format!("</{}>", tag_name);

            while self.position < self.input.len() {
                // Check for closing tag.
                if self.input[self.position..].starts_with(&closing) {
                    self.position += closing.len();
                    break;
                }

                if self.peek() == '<' {
                    if let Some(node) = self.parse_node() {
                        children.push(node);
                    }
                } else {
                    if let Some(text) = self.parse_text_node() {
                        children.push(Node::Text(text));
                    }
                }
            }

            Some(Node::Element(Element {
                tag_name,
                attributes,
                children,
                namespace: None,
            }))
        }
    }

    fn parse_text_node(&mut self) -> Option<Text> {
        let start = self.position;
        while self.position < self.input.len() {
            if self.input.as_bytes()[self.position] == b'<' {
                break;
            }
            self.position += 1;
        }
        if self.position == start {
            return None;
        }
        let content = self.input[start..self.position].to_string();
        Some(Text { content })
    }

    fn peek(&self) -> char {
        self.input[self.position..].chars().next().unwrap_or('\0')
    }

    fn advance(&mut self) {
        if self.position < self.input.len() {
            self.position += self.input[self.position..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        }
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() {
            let c = self.input.as_bytes()[self.position];
            if c.is_ascii_whitespace() {
                self.position += 1;
            } else {
                break;
            }
        }
    }

    fn skip_whitespace_attr(&mut self) {
        while self.position < self.input.len() {
            let c = self.input.as_bytes()[self.position];
            if c.is_ascii_whitespace() {
                self.position += 1;
            } else {
                break;
            }
        }
    }

    fn skip_until(&mut self, target: char) {
        while self.position < self.input.len() {
            if self.peek() == target {
                break;
            }
            self.advance();
        }
    }
}

impl Default for HtmlParser {
    fn default() -> Self {
        Self::new(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_html() {
        let html = "<html><body><p>hello</p></body></html>";
        let doc = HtmlParser::new(html.to_string()).parse();
        match &doc.children[0] {
            Node::Element(el) => assert_eq!(el.tag_name, "html"),
            _ => panic!("expected html element"),
        }
    }

    #[test]
    fn parses_attributes() {
        let html = "<a href=\"https://example.com\">link</a>";
        let doc = HtmlParser::new(html.to_string()).parse();
        match &doc.children[0] {
            Node::Element(el) => {
                assert_eq!(el.tag_name, "a");
                assert_eq!(el.attributes.len(), 1);
                assert_eq!(el.attributes[0].0, "href");
                assert_eq!(el.attributes[0].1, "https://example.com");
            }
            _ => panic!("expected element"),
        }
    }

    #[test]
    fn handles_self_closing() {
        let html = "<br>";
        let doc = HtmlParser::new(html.to_string()).parse();
        match &doc.children[0] {
            Node::Element(el) => {
                assert_eq!(el.tag_name, "br");
                assert!(el.children.is_empty());
            }
            _ => panic!("expected element"),
        }
    }

    #[test]
    fn parses_comments() {
        let html = "<!-- a comment -->";
        let doc = HtmlParser::new(html.to_string()).parse();
        match &doc.children[0] {
            Node::Comment(c) => assert_eq!(c.content, " a comment "),
            _ => panic!("expected comment"),
        }
    }
}
