# Contributing to NaBrowse

NaBrowse is an open-source, community-driven project. Contributions are
welcome — whether you're fixing typos, writing crates, or designing the
layout engine.

## How to contribute

1. Fork the repository on GitHub.
2. Create a feature branch: `git checkout -b feature/your-feature`
3. Make your changes.
4. Run tests: `cargo test`
5. Run clippy: `cargo clippy`
6. Format: `cargo fmt`
7. Commit and push: `git push origin feature/your-feature`
8. Open a pull request.

## Coding standards

- **No AI slop**: All code must be written or carefully reviewed by humans.
  No unreviewed LLM-generated code.
- **Privacy first**: No telemetry, no tracking, no phone-home. Ever.
- **Memory safety**: Rust throughout. No `unsafe` unless absolutely necessary
  and documented.
- **Tests**: Every crate should have unit tests for its core functionality.
- **Documentation**: Every public item should have doc comments.

## Code review

All pull requests require review. We look for:

- Correctness and safety
- Privacy implications
- Performance
- Code clarity

## License

By contributing, you agree that your contributions will be licensed under the
MIT or Apache-2.0 license (at the contributor's option).
