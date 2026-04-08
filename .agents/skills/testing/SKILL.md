---
name: testing
description: Test and verification guide for the project. Use when running tests, adding new tests, verifying changes, or preparing a submission checklist.
---

# Project Test and Verification Guide

## Verification Workflow

1. **Unit Tests**: Run unit tests for individual components using the project's test runner (e.g., `cargo test`, `npm test`).
2. **Integration Tests**: Run integration tests to verify component interactions.
3. **End-to-End (E2E) Tests**: Run E2E tests for full system verification.
4. **Manual Verification**: Perform manual checks for UI or complex workflows that are not fully automated.

## Core Commands

### Building the Project
Review the `README.md` for build instructions. Common commands:
```bash
cargo build
# or
npm install && npm run build
```

### Running Tests
Review the `README.md` for test instructions. Common commands:
```bash
cargo test
# or
npm test
```

## Adding New Tests

- **Unit Tests**: Add tests within the source directory of the relevant component using standard test annotations.
- **Integration Tests**: Place integration tests in a dedicated `tests/` directory.
- **E2E Tests**: Follow the project's convention for adding E2E or system-level tests.

## Pre-Submission Checklist

- [ ] All relevant components compile without warnings.
- [ ] All tests pass successfully.
- [ ] Any new feature has corresponding tests (unit, integration, or E2E).
- [ ] Changes have been verified in the appropriate environment.
- [ ] Documentation is updated if necessary.
