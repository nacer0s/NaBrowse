//! CSS parser and style computation for NaBrowse.
//!
//! Parses stylesheets into rules, computes specificity, applies the cascade,
//! and produces a computed style for every DOM node.
//!
//! Privacy: purely local computation, no network, no telemetry.

use nabrowse_shared::{Color, EdgeInsets};

/// A parsed CSS stylesheet: a list of rules.
#[derive(Debug, Clone, Default)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

/// A single CSS rule: selectors + declarations.
#[derive(Debug, Clone)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

/// A CSS selector.
#[derive(Debug, Clone)]
pub enum Selector {
    /// Universal selector `*`.
    Universal,
    /// Type selector `div`, `p`, etc.
    Type(String),
    /// Class selector `.foo`.
    Class(String),
    /// ID selector `#foo`.
    Id(String),
    /// Descendant combinator `a b`.
    Descendant(Box<Selector>, Box<Selector>),
    /// Attribute selector `[href]`.
    AttributeExists(String),
    /// Attribute selector `[href="value"]`.
    AttributeEquals(String, String),
}

/// A single CSS declaration: property + value.
#[derive(Debug, Clone)]
pub struct Declaration {
    pub property: String,
    pub value: Value,
}

/// A CSS value.
#[derive(Debug, Clone)]
pub enum Value {
    Keyword(String),
    Length(f32, Unit),
    Color(Color),
    Url(String),
    List(Vec<Value>),
}

/// CSS length unit.
#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    Px,
    Percent,
    Em,
    Rem,
}

/// Computed style for a single element after cascade + inheritance.
#[derive(Debug, Clone, Default)]
pub struct ComputedStyle {
    pub color: Option<Color>,
    pub background_color: Option<Color>,
    pub font_size: Option<f32>,
    pub font_family: Option<String>,
    pub margin: EdgeInsets,
    pub padding: EdgeInsets,
    pub border: EdgeInsets,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub display: Display,
    pub float: Float,
}

/// CSS display value.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Display {
    #[default]
    Inline,
    Block,
    InlineBlock,
    Flex,
    Grid,
    None,
}

/// CSS float value.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Float {
    #[default]
    None,
    Left,
    Right,
}

/// The CSS parser.
pub struct CssParser {
    input: String,
    position: usize,
}

impl CssParser {
    pub fn new(input: String) -> Self {
        Self { input, position: 0 }
    }

    /// Parse the full stylesheet.
    pub fn parse(mut self) -> Stylesheet {
        let mut rules = Vec::new();

        while self.position < self.input.len() {
            self.skip_whitespace();
            if self.position >= self.input.len() {
                break;
            }

            // Skip comments.
            if self.input[self.position..].starts_with("/*") {
                self.skip_comment();
                continue;
            }

            // Parse at-rule (e.g. @media) - skip for now.
            if self.peek() == '@' {
                self.skip_at_rule();
                continue;
            }

            // Parse selectors until '{'.
            let selectors = self.parse_selector_list();
            if self.peek() != '{' {
                // Malformed, skip to next '}'.
                self.skip_to('}');
                continue;
            }
            self.advance(); // consume '{'

            let declarations = self.parse_declarations();

            if self.peek() == '}' {
                self.advance(); // consume '}'
            }

            rules.push(Rule {
                selectors,
                declarations,
            });
        }

        Stylesheet { rules }
    }

    fn skip_comment(&mut self) {
        self.position += 2; // "/*"
        while self.position < self.input.len() {
            if self.input[self.position..].starts_with("*/") {
                self.position += 2;
                return;
            }
            self.position += 1;
        }
    }

    fn skip_at_rule(&mut self) {
        // Skip until ';' or '}'.
        while self.position < self.input.len() {
            let c = self.peek();
            if c == ';' || c == '}' {
                return;
            }
            self.advance();
        }
    }

    fn parse_selector_list(&mut self) -> Vec<Selector> {
        let mut selectors = Vec::new();
        loop {
            self.skip_whitespace();
            let sel = self.parse_selector();
            selectors.push(sel);
            self.skip_whitespace();
            if self.peek() == ',' {
                self.advance();
                continue;
            }
            break;
        }
        selectors
    }

    fn parse_selector(&mut self) -> Selector {
        self.skip_whitespace();

        // Universal.
        if self.peek() == '*' {
            self.advance();
            return Selector::Universal;
        }

        // ID selector.
        if self.peek() == '#' {
            self.advance();
            let name = self.read_identifier();
            return Selector::Id(name);
        }

        // Class selector.
        if self.peek() == '.' {
            self.advance();
            let name = self.read_identifier();
            return Selector::Class(name);
        }

        // Attribute selector.
        if self.peek() == '[' {
            self.advance();
            let attr = self.read_until(|c| c == '=' || c == ']' || c.is_ascii_whitespace());
            self.skip_whitespace();
            if self.peek() == '=' {
                self.advance();
                self.skip_whitespace();
                let quote = self.peek();
                let value = if quote == '"' || quote == '\'' {
                    self.advance();
                    let v = self.read_until(|c| c == quote);
                    if self.peek() == quote {
                        self.advance();
                    }
                    v
                } else {
                    self.read_until(|c| c == ']')
                };
                self.skip_to(']');
                if self.peek() == ']' {
                    self.advance();
                }
                return Selector::AttributeEquals(attr, value);
            }
            self.skip_to(']');
            if self.peek() == ']' {
                self.advance();
            }
            return Selector::AttributeExists(attr);
        }

        // Type selector or descendant combinator.
        let first = self.read_identifier();
        self.skip_whitespace();

        // Descendant combinator (whitespace followed by another selector).
        if self.peek() != '{' && self.peek() != ',' && self.peek() != '>' && self.peek() != '+' {
            let second = self.parse_selector();
            return Selector::Descendant(Box::new(Selector::Type(first)), Box::new(second));
        }

        Selector::Type(first)
    }

    fn read_identifier(&mut self) -> String {
        let start = self.position;
        while self.position < self.input.len() {
            let c = self.input.as_bytes()[self.position];
            if c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b'.' || c == b'#' {
                self.position += 1;
            } else {
                break;
            }
        }
        self.input[start..self.position].to_string()
    }

    fn read_until<F>(&mut self, pred: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let start = self.position;
        while self.position < self.input.len() {
            let c = self.input[self.position..].chars().next().unwrap_or('\0');
            if pred(c) {
                break;
            }
            self.position += c.len_utf8();
        }
        self.input[start..self.position].to_string()
    }

    fn skip_to(&mut self, target: char) {
        while self.position < self.input.len() {
            if self.peek() == target {
                break;
            }
            self.advance();
        }
    }

    fn parse_declarations(&mut self) -> Vec<Declaration> {
        let mut declarations = Vec::new();

        loop {
            self.skip_whitespace();
            if self.peek() == '}' || self.position >= self.input.len() {
                break;
            }

            // Property name.
            let prop_start = self.position;
            while self.position < self.input.len() {
                let c = self.input.as_bytes()[self.position];
                if c == b':' || c.is_ascii_whitespace() || c == b'}' {
                    break;
                }
                self.position += 1;
            }
            let property = self.input[prop_start..self.position].trim().to_string();

            self.skip_whitespace();
            if self.peek() != ':' {
                self.skip_to(';');
                if self.peek() == ';' {
                    self.advance();
                }
                continue;
            }
            self.advance(); // consume ':'
            self.skip_whitespace();

            // Value until ';'.
            let val_start = self.position;
            while self.position < self.input.len() {
                if self.peek() == ';' || self.peek() == '}' {
                    break;
                }
                self.advance();
            }
            let value_str = self.input[val_start..self.position].trim().to_string();

            if self.peek() == ';' {
                self.advance();
            }

            if !property.is_empty() {
                declarations.push(Declaration {
                    property,
                    value: parse_value(&value_str),
                });
            }
        }

        declarations
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

    fn peek(&self) -> char {
        self.input[self.position..].chars().next().unwrap_or('\0')
    }

    fn advance(&mut self) {
        if self.position < self.input.len() {
            self.position += self.input[self.position..].chars().next().map(|c| c.len_utf8()).unwrap_or(1);
        }
    }
}

/// Parse a CSS value string into a Value enum.
fn parse_value(s: &str) -> Value {
    let s = s.trim();
    if s.is_empty() {
        return Value::Keyword(String::new());
    }

    // Color keywords.
    match s {
        "inherit" | "initial" | "unset" => return Value::Keyword(s.to_string()),
        "none" => return Value::Keyword("none".to_string()),
        "left" => return Value::Keyword("left".to_string()),
        "right" => return Value::Keyword("right".to_string()),
        "block" => return Value::Keyword("block".to_string()),
        "inline" => return Value::Keyword("inline".to_string()),
        "flex" => return Value::Keyword("flex".to_string()),
        "grid" => return Value::Keyword("grid".to_string()),
        "inline-block" => return Value::Keyword("inline-block".to_string()),
        _ => {}
    }

    // Hex color #RRGGBB or #RGB.
    if s.starts_with('#') {
        if let Some(color) = parse_hex_color(s) {
            return Value::Color(color);
        }
    }

    // Length with unit (e.g. 16px, 100%, 1em).
    if let Some((num, unit)) = parse_length(s) {
        return Value::Length(num, unit);
    }

    Value::Keyword(s.to_string())
}

fn parse_hex_color(s: &str) -> Option<Color> {
    let hex = s.trim_start_matches('#');
    let (r, g, b) = if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        (r, g, b)
    } else if hex.len() == 3 {
        let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
        let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
        let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
        (r, g, b)
    } else {
        return None;
    };
    Some(Color::new(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        1.0,
    ))
}

fn parse_length(s: &str) -> Option<(f32, Unit)> {
    let s = s.trim();
    if s.ends_with("px") {
        let num = s[..s.len() - 2].trim().parse::<f32>().ok()?;
        return Some((num, Unit::Px));
    }
    if s.ends_with("%") {
        let num = s[..s.len() - 1].trim().parse::<f32>().ok()?;
        return Some((num, Unit::Percent));
    }
    if s.ends_with("em") {
        let num = s[..s.len() - 2].trim().parse::<f32>().ok()?;
        return Some((num, Unit::Em));
    }
    if s.ends_with("rem") {
        let num = s[..s.len() - 3].trim().parse::<f32>().ok()?;
        return Some((num, Unit::Rem));
    }
    None
}

/// Specificity of a selector: (a, b, c) where
/// a = ID count, b = class/attribute count, c = type/universal count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Specificity(pub u32, pub u32, pub u32);

impl Selector {
    /// Compute the specificity of this selector.
    pub fn specificity(&self) -> Specificity {
        match self {
            Selector::Id(_) => Specificity(1, 0, 0),
            Selector::Class(_) | Selector::AttributeExists(_) | Selector::AttributeEquals(_, _) => {
                Specificity(0, 1, 0)
            }
            Selector::Type(_) | Selector::Universal => Specificity(0, 0, 1),
            Selector::Descendant(a, b) => {
                let sa = a.specificity();
                let sb = b.specificity();
                Specificity(sa.0 + sb.0, sa.1 + sb.1, sa.2 + sb.2)
            }
        }
    }
}

/// The style engine: matches rules to elements and computes styles.
pub struct StyleEngine {
    stylesheet: Stylesheet,
}

impl StyleEngine {
    pub fn new(stylesheet: Stylesheet) -> Self {
        Self { stylesheet }
    }

    /// Compute the style for an element given its tag, classes, id, and parent
    /// computed style (for inheritance).
    pub fn compute(
        &self,
        tag: &str,
        classes: &[String],
        id: Option<&str>,
        parent: Option<&ComputedStyle>,
    ) -> ComputedStyle {
        let mut style = ComputedStyle::default();

        // Inherit from parent.
        if let Some(p) = parent {
            style.font_family = p.font_family.clone();
            style.color = p.color;
        }

        // Collect matching rules.
        let mut matches: Vec<(Specificity, &Declaration)> = Vec::new();
        for rule in &self.stylesheet.rules {
            for selector in &rule.selectors {
                if selector_matches(selector, tag, classes, id) {
                    let spec = selector.specificity();
                    for decl in &rule.declarations {
                        matches.push((spec, decl));
                    }
                }
            }
        }

        // Sort by specificity (ascending), so later/higher specificity wins.
        matches.sort_by(|a, b| a.0.cmp(&b.0));

        for (_spec, decl) in matches {
            apply_declaration(&mut style, decl);
        }

        style
    }
}

/// Check if a selector matches an element.
fn selector_matches(selector: &Selector, tag: &str, classes: &[String], id: Option<&str>) -> bool {
    match selector {
        Selector::Universal => true,
        Selector::Type(t) => t.eq_ignore_ascii_case(tag),
        Selector::Class(c) => classes.iter().any(|cl| cl == c),
        Selector::Id(i) => id.map_or(false, |i2| i2 == i),
        Selector::AttributeExists(attr) => {
            // Simplified: treat as matching if attr is "class" or "id".
            attr == "class" || attr == "id"
        }
        Selector::AttributeEquals(attr, val) => match attr.as_str() {
            "class" => classes.iter().any(|cl| cl == val),
            "id" => id.map_or(false, |i2| i2 == val),
            _ => false,
        },
        Selector::Descendant(ancestor, descendant) => {
            // Simplified descendant: match if descendant matches and ancestor
            // is Universal or Type matching tag.
            if !selector_matches(descendant, tag, classes, id) {
                return false;
            }
            match ancestor.as_ref() {
                Selector::Universal => true,
                Selector::Type(t) => t.eq_ignore_ascii_case(tag),
                _ => false,
            }
        }
    }
}

/// Apply a single declaration to a computed style.
fn apply_declaration(style: &mut ComputedStyle, decl: &Declaration) {
    let prop = decl.property.as_str();
    match prop {
        "color" => {
            if let Value::Color(c) = &decl.value {
                style.color = Some(*c);
            }
        }
        "background-color" | "background" => {
            if let Value::Color(c) = &decl.value {
                style.background_color = Some(*c);
            }
        }
        "font-size" => {
            if let Value::Length(n, Unit::Px) = &decl.value {
                style.font_size = Some(*n);
            }
        }
        "font-family" => {
            if let Value::Keyword(s) = &decl.value {
                style.font_family = Some(s.clone());
            }
        }
        "margin-top" => style.margin.top = length_px(&decl.value).unwrap_or(0.0),
        "margin-right" => style.margin.right = length_px(&decl.value).unwrap_or(0.0),
        "margin-bottom" => style.margin.bottom = length_px(&decl.value).unwrap_or(0.0),
        "margin-left" => style.margin.left = length_px(&decl.value).unwrap_or(0.0),
        "padding-top" => style.padding.top = length_px(&decl.value).unwrap_or(0.0),
        "padding-right" => style.padding.right = length_px(&decl.value).unwrap_or(0.0),
        "padding-bottom" => style.padding.bottom = length_px(&decl.value).unwrap_or(0.0),
        "padding-left" => style.padding.left = length_px(&decl.value).unwrap_or(0.0),
        "border-top" => style.border.top = length_px(&decl.value).unwrap_or(0.0),
        "border-right" => style.border.right = length_px(&decl.value).unwrap_or(0.0),
        "border-bottom" => style.border.bottom = length_px(&decl.value).unwrap_or(0.0),
        "border-left" => style.border.left = length_px(&decl.value).unwrap_or(0.0),
        "width" => {
            if let Value::Length(n, Unit::Px) = &decl.value {
                style.width = Some(*n);
            }
        }
        "height" => {
            if let Value::Length(n, Unit::Px) = &decl.value {
                style.height = Some(*n);
            }
        }
        "display" => {
            if let Value::Keyword(s) = &decl.value {
                style.display = match s.as_str() {
                    "block" => Display::Block,
                    "inline" => Display::Inline,
                    "inline-block" => Display::InlineBlock,
                    "flex" => Display::Flex,
                    "grid" => Display::Grid,
                    "none" => Display::None,
                    _ => Display::Inline,
                };
            }
        }
        "float" => {
            if let Value::Keyword(s) = &decl.value {
                style.float = match s.as_str() {
                    "left" => Float::Left,
                    "right" => Float::Right,
                    _ => Float::None,
                };
            }
        }
        _ => {}
    }
}

fn length_px(value: &Value) -> Option<f32> {
    match value {
        Value::Length(n, Unit::Px) => Some(*n),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_rule() {
        let css = "p { color: #ff0000; font-size: 16px; }";
        let sheet = CssParser::new(css.to_string()).parse();
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].selectors.len(), 1);
        assert_eq!(sheet.rules[0].declarations.len(), 2);
    }

    #[test]
    fn specificity_id_beats_class() {
        let id_spec = Selector::Id("foo".to_string()).specificity();
        let class_spec = Selector::Class("bar".to_string()).specificity();
        assert!(id_spec > class_spec);
    }

    #[test]
    fn computes_inherited_color() {
        let css = "body { color: #000000; } p { color: #ff0000; }";
        let sheet = CssParser::new(css.to_string()).parse();
        let engine = StyleEngine::new(sheet);

        let parent = engine.compute("body", &[], None, None);
        let child = engine.compute("p", &[], None, Some(&parent));

        assert_eq!(child.color, Some(Color::new(1.0, 0.0, 0.0, 1.0)));
    }
}
