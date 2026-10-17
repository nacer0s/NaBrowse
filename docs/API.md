# API reference — NaBrowse

## nabrowse-shared

### Geometry types

- `Point { x: f32, y: f32 }` — 2D point.
- `Size { width: f32, height: f32 }` — 2D size.
- `Rect { origin: Point, size: Size }` — rectangle with helper methods
  (`min_x`, `max_x`, `intersection`, `is_empty`).
- `Color { r, g, b, a }` — RGBA color (0.0..=1.0).
- `EdgeInsets { top, left, bottom, right }` — padding/margin insets.

### URL

- `NabrowseUrl::parse(input: &str) -> Option<NabrowseUrl>` — parse a URL string.
- `NabrowseUrl::to_string()` — serialize back to string.

## nabrowse-network

### NetworkClient

- `NetworkClient::new() -> NetworkResult<Self>` — create client with system root certificates.
- `NetworkClient::fetch(url: &str) -> NetworkResult<FetchResponse>` — fetch a URL (HTTP/1.1, TLS optional).

### FetchResponse

- `status: u16` — HTTP status code.
- `headers: Vec<(String, String)>` — response headers.
- `body: Vec<u8>` — response body bytes.
- `url: String` — requested URL.

## nabrowse-html

### DOM

- `Node::Element(Element)`, `Node::Text(Text)`, `Node::Comment(Comment)`, `Node::Document(Document)`.
- `HtmlParser::new(input: String)` → `HtmlParser`
- `HtmlParser::parse() -> Document` — parse HTML into a DOM tree.

## nabrowse-css

- `Stylesheet { rules: Vec<Rule> }`
- `Rule { selectors: Vec<Selector>, declarations: Vec<Declaration> }`
- `Selector::Universal | Type(String) | Class(String) | Id(String) | ...`
- `Declaration { property: String, value: Value }`
- `Value::Keyword(String) | Length(f32, Unit) | Color(Color) | ...`
- `ComputedStyle` — final computed style for an element.
- `StyleEngine::new(stylesheet)` → compute styles for elements.

## nabrowse-layout

- `LayoutEngine::new(viewport_width, viewport_height)` → `LayoutEngine`
- `LayoutEngine::layout(document_node: &Node) -> LayoutBox` — layout the DOM tree.
- `LayoutBox { node, style, dimensions, children }`
- `Dimensions { content, padding, border, margin: Rect }`

## nabrowse-paint

- `GraphicsContext { surface, device, queue, config, size }`
- `GraphicsContext::from_window(window: &dyn Window)` — create from winit window.
- `GraphicsContext::resize(width, height)` — resize the swap chain.

## nabrowse-javascript

- `JsValue::Undefined | Null | Boolean | Number | String | Object | Function`
- `JsEngine::new()` → `JsEngine`
- `JsEngine::execute(source: &str) -> Result<JsValue, JsError>` — run JS source.
