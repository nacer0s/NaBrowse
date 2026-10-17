# Roadmap — NaBrowse

NaBrowse is developed in phases. There is no deadline — this is a community,
open-source, "for fun" project.

## Phase 0 — Scaffolding (current)

- [x] Rust workspace with 8 crates
- [x] Cross-platform window via winit
- [x] GPU render loop via wgpu 30
- [x] Clear color frame presentation
- [x] Unit tests in all crates

## Phase 1 — Networking

- [x] HTTP/1.1 client (GET)
- [x] TLS via rustls (HTTPS)
- [x] DNS resolution via OS
- [x] Fetch API: URL → full response (headers + body)

## Phase 2 — HTML parser + DOM

- [x] HTML tokenizer (tags, attributes, text, comments, self-closing)
- [x] Tree builder with error recovery
- [x] DOM tree in memory
- [x] Void element handling

## Phase 3 — CSS parser + styles

- [x] CSS parser (selectors, declarations, values)
- [x] Specificity calculation
- [x] Cascade (later specificity wins)
- [x] Inheritance from parent
- [x] Computed style per element

## Phase 4 — Layout engine

- [x] Box model (margin, border, padding, content)
- [x] Block layout (normal flow)
- [x] Inline layout (basic text)
- [ ] Flexbox
- [ ] CSS Grid
- [ ] Text wrapping and line breaking

## Phase 5 — Paint pipeline

- [ ] Rectangles, borders, backgrounds
- [ ] Text rendering (glyphs)
- [ ] Image decoding (PNG, JPEG)
- [ ] GPU shader pipeline

## Phase 6 — Browser shell

- [ ] Tabs
- [ ] URL bar / address bar
- [ ] Back / forward / refresh
- [ ] Multiple windows
- [ ] History and bookmarks

## Phase 7 — JavaScript engine

- [x] Lexer (tokens: numbers, strings, identifiers, keywords, punctuators)
- [x] Recursive-descent parser (expressions, statements, blocks, if/while)
- [x] Tree-walk interpreter (variables, arithmetic, comparison, logic)
- [ ] DOM API bridge (`document`, `window`, `addEventListener`)
- [ ] Event loop integration

## Phase 8 — Modern web features

- [ ] HTTP/2, HTTP/3
- [ ] Cookies, localStorage, cache
- [ ] CSS Flexbox and Grid
- [ ] Forms and form validation
- [ ] Video/audio
- [ ] DevTools

## Phase 9 — "Fully usable" polish

- [ ] Performance optimization
- [ ] Security hardening
- [ ] Packaging for Windows, macOS, Linux
- [ ] Community contributions

## What "fully usable" means

A browser that can load and render real-world websites (including those that
require JavaScript, CSS Grid/Flexbox, HTTPS, cookies, etc.). This is a
multi-year effort; contributions are welcome.
