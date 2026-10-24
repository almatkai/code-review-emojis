# Code Review Emoji Guide — Native V2

A standalone Rust library and compiled-in Tezbar adapter for the original Code Review Emoji Guide extension by **rus64**, based on the guide by [erikthedeveloper](https://github.com/erikthedeveloper/code-review-emoji-guide).

`index` (Show Review Emoji) displays the original ten emoji/title pairs in order under **Emojis**, followed by the book-icon guide link under **More**. The search placeholder is **Search Emojis**. Each emoji's primary action pastes its exact Unicode content. The guide action opens its fixed HTTPS URL. There are no preferences or copy actions.

The V2 manifest is `com.tezbar.native.code-review-emojis`, replacing `raycast.code-review-emojis`. Intended platforms are macOS and Windows. Both verification records remain **not-run**: headless tests and cross-compilation do not verify real window focus, clipboard permissions, paste delivery, browser launching or UI appearance.

## Integration

This package targets Tezbar beta commit `fd300e276356587726566026f072563eb9c34cb1`. It requires compiled registration and a host rebuild. Follow [INTEGRATION.md](INTEGRATION.md), including stable vendoring at `src-tauri/native-extensions/code-review-emojis` and copying the icon to `src/renderer/public/extensions/icons/code-review-emojis.png`.

The manifest is metadata. It cannot load Rust code from an external directory or install executable dispatch by writing a JSON marker.

## Behavior and beta adaptations

The beta SDK's `open` function uses `shell:open`. Paste uses `CoreContext::request("core:platform:paste-text", [emoji])`, as the beta SDK has no paste helper. That platform route already writes the clipboard, hides Tezbar and sends the platform paste shortcut; the adapter performs no extra focus or clipboard operations.

Paste actions use the supported renderer action kind `action` with label **Paste**. Every row has one primary action ID. No `copyText` is supplied, so the renderer invokes the adapter. The complete list stays in the client; beta's actual List parser filters title, subtitle and section case-insensitively with trimmed queries. The source defines no row keywords. Beta does not search icon glyphs or support a `keywords` row prop; no unsupported props or extra search data are added.

Each launch creates a fresh random generation. The host keeps private session state and serializes access. Only exact IDs issued by that launch are accepted; disposal clears the generation. The command and operation are checked before any effect. Payload content, URLs, preferences and replacement state are ignored. Routes receive only predefined source data.

Transport failures and error/malformed envelopes propagate as errors; an effect needs `ok: true` without an error to acknowledge success. There is no automatic retry because a disconnected transport may already have performed the OS effect. An explicit retry can reuse the current action; successful actions also remain available for repeated selections. The adapter returns the SDK's unchanged-view envelope after acknowledgement.

## Verification

From this directory:

```sh
cargo test --locked
cargo fmt -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo +1.85.0 test --locked
cargo +1.85.0 fmt -- --check
cargo +1.85.0 clippy --locked --all-targets -- -D warnings
cargo check --locked --all-targets --target x86_64-pc-windows-gnu
cargo clippy --locked --all-targets --target x86_64-pc-windows-gnu -- -D warnings
pnpm --dir tools install --frozen-lockfile --ignore-scripts
pnpm --dir tools test
cargo package --locked
```

The five adapter contract tests compile `src/tezbar_adapter.rs` unchanged with byte-exact beta `common.rs` and `v2_sdk.rs`; their five original tests run too. Only the router and unused desktop event API are stubbed. No test invokes a real clipboard, window, paste or browser operation. Tests cover all ten paste contents, the guide URL, error/retry behavior, command/operation validation, cross-session and stale/fabricated actions, disposal and relaunch.

The Node verification executes pinned original TSX with element-only mocks and checks its capture against the Rust output. It also executes the actual beta List parser/filter function ASTs, validates the full beta JSON Schema, and checks provenance, licenses and the byte-identical icon. Node dependencies are verification tools only; Rust runtime dependencies are `serde_json` and `getrandom`, with MSRV 1.85 and a locked dependency graph.

See [provenance.json](provenance.json) for source commits, paths and hashes. The package includes the original MIT license and beta Apache-2.0 license; packaging SPDX is **MIT AND Apache-2.0**. Attribution is retained in the description, provenance and [NOTICE](NOTICE), since the beta schema forbids a root `author` field.
