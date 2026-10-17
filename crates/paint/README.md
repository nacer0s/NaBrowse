# nabrowse-paint

GPU paint pipeline for NaBrowse (wgpu).

Wraps wgpu device, queue, surface, and swap chain. Provides resize and frame
presentation. This is the foundation the layout/paint engine draws into.

## Features

- wgpu GPU context (device, queue, surface)
- Cross-platform GPU abstraction (Vulkan, Metal, DX12, OpenGL, WebGPU)
- Surface resize handling
- Frame presentation via `queue.present()`

## License

MIT OR Apache-2.0.
