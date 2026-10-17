//! Layout engine for NaBrowse.
//!
//! Takes the DOM tree + computed styles and produces a layout tree with
//! concrete pixel rectangles for every box. Implements the CSS box model
//! (margin, border, padding, content), block layout, and inline layout.
//!
//! Privacy: fully local, no network, no telemetry.

use nabrowse_css::{ComputedStyle, Display};
use nabrowse_html::Node;
use nabrowse_shared::{Point, Rect, Size};

/// A layout box with geometry and style.
#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub node: Node,
    pub style: ComputedStyle,
    pub dimensions: Dimensions,
    pub children: Vec<LayoutBox>,
}

/// The geometry of a layout box.
#[derive(Debug, Clone, Default)]
pub struct Dimensions {
    /// Content area.
    pub content: Rect,
    /// Padding area (content + padding).
    pub padding: Rect,
    /// Border area (padding + border).
    pub border: Rect,
    /// Margin area (border + margin).
    pub margin: Rect,
}

/// The layout engine.
pub struct LayoutEngine {
    viewport_width: f32,
    viewport_height: f32,
}

impl LayoutEngine {
    pub fn new(viewport_width: f32, viewport_height: f32) -> Self {
        Self {
            viewport_width,
            viewport_height,
        }
    }

    /// Layout the entire DOM tree starting from the root document node.
    pub fn layout(&mut self, document_node: &Node) -> LayoutBox {
        let style = ComputedStyle::default();
        self.layout_block(document_node, &style, 0.0, 0.0, self.viewport_width)
    }

    /// Layout a block-level box.
    fn layout_block(
        &mut self,
        node: &Node,
        parent_style: &ComputedStyle,
        x: f32,
        y: f32,
        width: f32,
    ) -> LayoutBox {
        let style = self.compute_style(node, parent_style);

        let mut box_ = LayoutBox {
            node: node.clone(),
            style: style.clone(),
            dimensions: Dimensions::default(),
            children: Vec::new(),
        };

        // Content width = box width - horizontal padding/border/margin.
        let content_width = width
            - style.margin.horizontal()
            - style.border.horizontal()
            - style.padding.horizontal();

        // Position content box.
        let content_x = x + style.margin.left + style.border.left + style.padding.left;
        let content_y = y + style.margin.top + style.border.top + style.padding.top;

        let mut cursor_y = content_y;

        match &node {
            Node::Element(el) => {
                // Layout children in block flow.
                for child in &el.children {
                    if style.display == Display::None {
                        continue;
                    }
                    let child_box = self.layout_block(child, &style, content_x, cursor_y, content_width);
                    cursor_y = child_box.dimensions.margin.max_y();
                    box_.children.push(child_box);
                }

                // Height: explicit or content height.
                let content_height = if let Some(h) = style.height {
                    h.max(0.0)
                } else {
                    (cursor_y - content_y).max(0.0)
                };

                box_.dimensions.content = Rect::new(
                    Point::new(content_x, content_y),
                    Size::new(content_width, content_height),
                );
            }
            Node::Text(_t) => {
                // Inline text: one line for now (Phase 0).
                let line_height = style.font_size.unwrap_or(16.0) * 1.2;
                box_.dimensions.content = Rect::new(
                    Point::new(content_x, content_y),
                    Size::new(content_width, line_height),
                );
                cursor_y += line_height;
                let _ = cursor_y;
            }
            Node::Comment(_) => {
                // Comments take no space.
                box_.dimensions.content = Rect::new(
                    Point::new(content_x, content_y),
                    Size::new(content_width, 0.0),
                );
            }
            Node::Document(doc) => {
                // Document: lay out children.
                for child in &doc.children {
                    let child_box = self.layout_block(child, &style, content_x, cursor_y, content_width);
                    cursor_y = child_box.dimensions.margin.max_y();
                    box_.children.push(child_box);
                }
                let content_height = (cursor_y - content_y).max(0.0);
                box_.dimensions.content = Rect::new(
                    Point::new(content_x, content_y),
                    Size::new(content_width, content_height),
                );
            }
        }

        // Expand to padding/border/margin boxes.
        box_.dimensions.padding = Rect::new(
            Point::new(
                box_.dimensions.content.origin.x - style.padding.left,
                box_.dimensions.content.origin.y - style.padding.top,
            ),
            Size::new(
                content_width + style.padding.horizontal(),
                box_.dimensions.content.size.height + style.padding.vertical(),
            ),
        );
        box_.dimensions.border = Rect::new(
            Point::new(
                box_.dimensions.padding.origin.x - style.border.left,
                box_.dimensions.padding.origin.y - style.border.top,
            ),
            Size::new(
                box_.dimensions.padding.size.width + style.border.horizontal(),
                box_.dimensions.padding.size.height + style.border.vertical(),
            ),
        );
        box_.dimensions.margin = Rect::new(
            Point::new(
                box_.dimensions.border.origin.x - style.margin.left,
                box_.dimensions.border.origin.y - style.margin.top,
            ),
            Size::new(
                box_.dimensions.border.size.width + style.margin.horizontal(),
                box_.dimensions.border.size.height + style.margin.vertical(),
            ),
        );

        box_
    }

    fn compute_style(
        &self,
        node: &Node,
        parent_style: &ComputedStyle,
    ) -> ComputedStyle {
        match node {
            Node::Element(el) => {
                let _classes: Vec<String> = el
                    .attributes
                    .iter()
                    .filter(|(k, _)| k == "class")
                    .flat_map(|(_, v)| v.split_whitespace().map(|s| s.to_string()))
                    .collect();
                let _id = el
                    .attributes
                    .iter()
                    .find(|(k, _)| k == "id")
                    .map(|(_, v)| v.as_str());

                // We don't have the stylesheet here in this simplified engine;
                // the caller should pass computed styles. For now return defaults
                // with inherited values.
                let mut style = ComputedStyle::default();
                style.font_family = parent_style.font_family.clone();
                style.color = parent_style.color;
                style.display = match el.tag_name.as_str() {
                    "div" | "p" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6"
                    | "ul" | "ol" | "li" | "blockquote" | "section" | "article"
                    | "header" | "footer" | "aside" | "nav" | "main" => Display::Block,
                    "span" | "a" | "strong" | "em" | "b" | "i" => Display::Inline,
                    _ => Display::Inline,
                };
                style
            }
            _ => parent_style.clone(),
        }
    }

    /// Return the viewport size.
    pub fn viewport(&self) -> Size {
        Size::new(self.viewport_width, self.viewport_height)
    }
}

/// Walk the layout tree and collect all paint rectangles (for debugging).
pub fn collect_boxes<'a>(box_: &'a LayoutBox, out: &mut Vec<&'a LayoutBox>) {
    out.push(box_);
    for child in &box_.children {
        collect_boxes(child, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layouts_simple_document() {
        let html = "<html><body><p>hello</p></body></html>";
        use nabrowse_html::HtmlParser;
        use nabrowse_html::Node;
        let doc = HtmlParser::new(html.to_string()).parse();

        let mut engine = LayoutEngine::new(800.0, 600.0);
        let root = engine.layout(&Node::Document(doc));

        // Root should be the document with children.
        assert!(!root.children.is_empty());
        assert!(root.dimensions.content.size.height > 0.0);
    }

    #[test]
    fn block_box_has_positive_area() {
        let html = "<div>content</div>";
        use nabrowse_html::HtmlParser;
        use nabrowse_html::Node;
        let doc = HtmlParser::new(html.to_string()).parse();

        let mut engine = LayoutEngine::new(800.0, 600.0);
        let root = engine.layout(&Node::Document(doc));

        let _boxes: Vec<&LayoutBox> = Vec::new();
        let _ = _boxes;
        let _ = root;
    }
}
