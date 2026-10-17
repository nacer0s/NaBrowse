# nabrowse-browser-shell

Browser shell for NaBrowse: window, event loop, and application entry point.

Creates a real OS window via winit, initializes a wgpu GPU context, and runs
the main event loop using winit 0.31's `ApplicationHandler` pattern.

## Binary

`cargo run` builds and runs the `nabrowse` binary, which opens a window and
renders the NaBrowse background color.

## License

MIT OR Apache-2.0.
