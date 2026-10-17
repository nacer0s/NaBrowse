# Building NaBrowse

## Prerequisites

- Rust (1.82 or later) — install via [rustup](https://rustup.rs/)
- A C compiler (for some dependencies)
- On Linux: `libwayland-dev`, `libxkbcommon-dev`, `libgtk-3-dev` (or
  equivalent for your display server)
- On macOS: Xcode command-line tools
- On Windows: Visual Studio with C++ build tools

## Clone

```bash
git clone https://github.com/nacer0s/NaBrowse.git
cd NaBrowse
```

## Build

```bash
# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run
cargo run
```

## Test

```bash
cargo test
```

## Cross-platform

NaBrowse targets Windows, macOS, and Linux from the same codebase. winit
handles windowing and wgpu handles GPU abstraction (Vulkan, Metal, DX12,
OpenGL, WebGPU).

## Troubleshooting

### wgpu adapter not found

Ensure your GPU drivers are up to date and Vulkan/Metal/DX12 is available.

### Build fails on raw-window-handle

Ensure you have the system development packages for your display server
(Wayland/X11 on Linux).
