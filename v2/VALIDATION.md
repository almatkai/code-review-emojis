# Validation record

Validated on 2026-10-24 with Rust 1.98.0 (aarch64-apple-darwin), Rust 1.85.0, and pnpm 11.23.0. Dependencies were locked. This record describes headless automated validation; macOS and Windows real OS/UI verification remains **not-run**.

- `cargo test --locked`: 10 passed (5 adapter contract tests and 5 tests from unchanged beta common/SDK fixtures).
- `cargo +1.85.0 test --locked`: 10 passed.
- `cargo fmt -- --check` and `cargo +1.85.0 fmt -- --check`: passed. Byte-exact beta modules are explicitly excluded from reformatting.
- `cargo clippy --locked --all-targets -- -D warnings`, on both Rust 1.98.0 and 1.85.0: passed.
- `cargo check --locked --all-targets --target x86_64-pc-windows-gnu` and corresponding strict Clippy, on both Rust 1.98.0 and 1.85.0: passed. These checks do not execute or link a Windows application.
- `pnpm --dir tools install --frozen-lockfile --ignore-scripts` and `pnpm --dir tools test`: passed. This executes the pinned original TSX oracle and the beta List parser/filter AST functions, checks all rows/primary actions/client search, checks supported action kinds against the actual beta shared type, validates the complete beta JSON Schema, and verifies pinned fixture hashes, license copies and icon size/hash.
- `cargo package --locked --allow-dirty`: passed with normal Cargo verification. Package includes the production adapter, all required fixtures, icon, notices and licenses; it excludes build outputs and Node dependencies.
- Tests from the extracted `.crate`: 10 passed on Rust 1.98.0 and 10 on Rust 1.85.0; locked Node installation and the source/schema/renderer/provenance verification passed using only extracted files.

No tests interact with a real clipboard, launcher window, paste target, browser, installed extension profile or live host. Router and desktop event stubs isolate effects. Full host registration/rebuild, end-to-end installation and real UI/OS checks still need to be performed as described in INTEGRATION.md.
