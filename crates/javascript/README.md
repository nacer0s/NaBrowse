# nabrowse-javascript

JavaScript engine for NaBrowse.

A from-scratch JavaScript engine with a lexer, recursive-descent parser, and
tree-walk interpreter.

## Features

- Lexer: numbers, strings, identifiers, keywords, punctuators
- Parser: expressions, statements, blocks, if/else, while loops
- Interpreter: variables, arithmetic, comparison, logical operators
- Tree-walk execution (no bytecode yet)

## Example

```rust
use nabrowse_javascript::JsEngine;

let mut engine = JsEngine::new();
let result = engine.execute("1 + 2 * 3;").unwrap();
// result == JsValue::Number(7.0)
```

## License

MIT OR Apache-2.0.
