# Contributing to Atlas

Thank you for your interest in contributing to Atlas! This document provides guidelines for contributing to the project.

## Code of Conduct

We are committed to providing a welcoming and inclusive environment. Please be respectful and professional in all interactions.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/atlas.git`
3. Create a feature branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Run tests: `cargo test`
6. Run linting: `cargo clippy -- -D warnings`
7. Format code: `cargo fmt`
8. Commit your changes with clear messages
9. Push to your fork: `git push origin feature/your-feature-name`
10. Open a Pull Request

## Development Guidelines

### Rust Code Standards

Atlas follows the [Microsoft Rust Guidelines](https://microsoft.github.io/rust-guidelines/). Key principles:

- Use the builder pattern for complex structs (TR-6.2)
- Wrap primitives in newtype wrappers (TR-6.3)
- Use `Result<T, E>` for fallible operations (TR-6.4)
- Use `anyhow::Error` with context (TR-6.5)
- Map external errors to domain errors (TR-6.6)
- Main returns `Result` (TR-6.7)
- Implement `Default` trait where appropriate (TR-6.8)
- Prefer `&str` over `String` where possible (TR-6.9)
- Use `if let`/`while let` for conditionals (TR-6.10)

### Testing

- Write unit tests alongside implementation (TDD approach)
- Maintain test coverage > 70%
- Add integration tests for new features
- Test error scenarios

### Documentation

- Document all public APIs with rustdoc comments
- Include examples in documentation
- Update relevant guides when adding features
- Document error conditions and panics

### Commit Messages

Use clear, descriptive commit messages:

```
feat: Add support for OpenID Connect authentication
fix: Handle Cosmos DB throttling errors correctly
docs: Update configuration guide with new options
test: Add integration tests for incremental export
```

## Pull Request Process

1. Ensure all tests pass
2. Update documentation as needed
3. Add entries to CHANGELOG.md
4. Request review from maintainers
5. Address review feedback
6. Squash commits if requested

## Adding New OpenEHR Vendors

To add support for a new OpenEHR vendor:

1. Implement the `OpenEhrVendor` trait in `src/adapters/openehr/vendor/`
2. Add vendor-specific models if needed
3. Update configuration schema to support the new vendor
4. Add integration tests
5. Document the new vendor in the user guide

## Questions?

Feel free to open an issue for questions or discussion before starting work on major features.

## License

By contributing to Atlas, you agree that your contributions will be licensed under the MIT License.

