# nabrowse-html

HTML parser and DOM tree for NaBrowse.

A from-scratch HTML tokenizer and tree builder that produces an in-memory DOM
tree. Handles tags, attributes, text, comments, self-closing tags, and basic
error recovery for unclosed tags.

## Example

```rust
use nabrowse_html::HtmlParser;

let html = "<html><body><p>hello</p></body></html>";
let doc = HtmlParser::new(html.to_string()).parse();
```

## License

MIT OR Apache-2.0.
