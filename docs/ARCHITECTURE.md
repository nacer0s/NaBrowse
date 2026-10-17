# Architecture — NaBrowse

## Overview

NaBrowse follows a simplified multi-process-inspired architecture (currently
single-process, with process separation planned for a later phase). The
browser is organized as a Rust workspace with eight crates.

## Crate dependency graph

```
nabrowse-browser-shell  (entry point: window, tabs, UI)
    ├── nabrowse-network   (HTTP, TLS, DNS, fetch)
    ├── nabrowse-html      (HTML parser, DOM)
    ├── nabrowse-css       (CSS parser, cascade, computed styles)
    ├── nabrowse-layout    (box model, block/inline layout)
    ├── nabrowse-paint     (wgpu GPU context, surface, frames)
    │     └── nabrowse-layout
    ├── nabrowse-javascript (JS lexer, parser, interpreter)
    └── nabrowse-shared     (Point, Size, Rect, Color, EdgeInsets, URL)
```

`nabrowse-shared` has no dependencies and is used by every other crate.

## Rendering pipeline

1. **Network**: Fetch HTML bytes from a URL via HTTP/1.1 (+TLS).
2. **HTML parser**: Tokenize bytes into a DOM tree (`nabrowse-html`).
3. **CSS parser**: Parse stylesheets into rules (`nabrowse-css`).
4. **Style engine**: Match selectors to DOM nodes, compute styles (cascade,
   specificity, inheritance).
5. **Layout engine**: Compute pixel rectangles for every DOM node using the
   CSS box model (`nabrowse-layout`).
6. **Paint**: Issue wgpu draw commands to render rectangles, text, and images
   (`nabrowse-paint`).
7. **Shell**: winit window, event loop, swap chain presentation.

## Privacy by design

The network crate is isolated: it only connects to user-requested URLs. No
telemetry, no tracking, no phone-home. All processing is local.

## Build

```bash
cargo build --release
```
