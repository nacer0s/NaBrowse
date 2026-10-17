# NaBrowse

**NaBrowse** is a free, open-source web browser engine built from scratch in Rust.
Privacy-first. No telemetry. No tracking. No phone-home.

## Project goals

- **From scratch**: HTML parser, CSS engine, layout engine, paint pipeline, and
  networking stack written in Rust — no Chromium, no WebKit, no Gecko.
- **Privacy protected**: No telemetry, no tracking, no user data collection,
  no phone-home. The browser only connects to URLs you explicitly request.
- **Modern and fast**: GPU-accelerated rendering via wgpu, memory-safe Rust
  throughout, designed for performance.
- **Cross-platform**: Windows, macOS, and Linux from a single codebase.
- **Open source**: MIT/Apache-2.0 licensed. Community-driven.

## Current status (Phase 0)

NaBrowse is in early development. The current milestone is a working window
with a GPU render loop:

- ✅ Cross-platform window (winit)
- ✅ GPU rendering context (wgpu 30)
- ✅ Render loop with clear color
- ✅ HTML parser with DOM tree
- ✅ CSS parser with cascade, specificity, and computed styles
- ✅ Layout engine (box model, block/inline layout)
- ✅ JavaScript engine (lexer, parser, tree-walk interpreter)
- ✅ Networking stack (HTTP/1.1, TLS via rustls)
- ✅ Unit tests across all crates

## Crates

| Crate | Purpose |
|-------|---------|
| `nabrowse-shared` | Geometry types (Point, Size, Rect, Color, EdgeInsets) and URL |
| `nabrowse-network` | HTTP/1.1 client, TLS, DNS, fetch |
| `nabrowse-html` | HTML tokenizer, tree builder, DOM |
| `nabrowse-css` | CSS parser, selectors, cascade, computed styles |
| `nabrowse-layout` | Box model, block layout, inline layout |
| `nabrowse-paint` | wgpu GPU context, surface, frame presentation |
| `nabrowse-javascript` | JS lexer, recursive-descent parser, tree-walk interpreter |
| `nabrowse-browser-shell` | Application entry point, winit window, event loop |

## Build

```bash
cargo build
cargo run
cargo test
cargo build --release
```

## Privacy

NaBrowse is designed to protect your privacy:

- **No telemetry**: We never send data about your browsing to any server.
- **No tracking**: No cookies, no localStorage, no fingerprinting (yet — these
  will be opt-in when implemented).
- **No phone-home**: The network stack only connects to URLs you explicitly
  navigate to.
- **Local-first**: All parsing, layout, painting, and JS execution happens
  locally on your machine.

See [`docs/PRIVACY.md`](docs/PRIVACY.md) for the full privacy policy.

## Roadmap

See [`docs/ROADMAP.md`](docs/ROADMAP.md) for the phased development plan.

## Architecture

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the system design.

## Contributing

See [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md).

## License

MIT OR Apache-2.0.
