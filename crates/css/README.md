# nabrowse-css

CSS parser and style computation for NaBrowse.

Parses stylesheets into rules, computes specificity, applies the cascade, and
produces computed styles for DOM nodes.

## Features

- CSS selector parsing (type, class, ID, attribute, universal, descendant)
- Specificity calculation
- Cascade (later/higher specificity wins)
- Inheritance from parent elements
- Computed styles (color, font-size, margin, padding, border, display, etc.)

## License

MIT OR Apache-2.0.
