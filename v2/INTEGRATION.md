# Compiled-in integration with Tezbar beta

Target: `fd300e276356587726566026f072563eb9c34cb1`. These are integration instructions; this repository does not modify the host or an installed profile.

1. Vendor the **contents of this `v2/` directory** into the host at `src-tauri/native-extensions/code-review-emojis/`. This directory must contain `Cargo.toml`, `src/lib.rs`, `src/tezbar_adapter.rs`, the manifest, icon, notices and fixtures/licenses. Commit the vendor copy with the host so the build has a stable source path.
2. Add the dependency in the host's `src-tauri/Cargo.toml` and update its lockfile:

   ```toml
   code-review-emojis = { path = "native-extensions/code-review-emojis" }
   ```

3. In `src-tauri/src/native_core/extensions/mod.rs`, register the production adapter unchanged:

   ```rust
   mod v2_code_review_emojis {
       include!(concat!(
           env!("CARGO_MANIFEST_DIR"),
           "/native-extensions/code-review-emojis/src/tezbar_adapter.rs"
       ));
   }
   ```

   The surrounding extensions module supplies `v2_sdk`; the host supplies `crate::native_core::common`. The adapter uses the vendored dependency for its independent logic.

4. Extend all three dispatch tables in `src-tauri/src/native_core/extensions/v2_runtime.rs`:

   - Add `"code-review-emojis"` to `slug`'s accepted slugs (canonical IDs, `raycast.` aliases and bare slugs then normalize through the existing logic).
   - Add `"code-review-emojis" => super::v2_code_review_emojis::commands(),` to `commands`.
   - Add this arm to `handle`:

     ```rust
     "code-review-emojis" => {
         super::v2_code_review_emojis::handle(ctx, command, operation, payload, state).await
     }
     ```

5. Append the exact object from `manifest.json` to the array in `src-tauri/src/native_core/extensions/v2_catalog.json`. Keep both OS verification records `not-run`. Validate the object against `schemas/extension-v2.schema.json`; do not add a root `author` field. The catalog derives runtime coverage from the compiled dispatch and declared platforms.
6. Copy `assets/code-review-emojis.png` unchanged to **`src/renderer/public/extensions/icons/code-review-emojis.png`**. The manifest URL is `/extensions/icons/code-review-emojis.png`. Expected size: 61058 bytes; SHA-256: `bd624e90c496cd28413add5990d3df7c31688425dd88aa8a7aef6ea205a8b41c`.
7. Run the host's required Rust/frontend checks and rebuild the application on each target OS. Check the compiled CLI's `scripts/extensions-v2-list.mjs` output for actual command coverage, then use the host's existing compiled-native installation flow. The native session layer still requires an installed command and owns session state, cancellation, disposal and serialization.
8. On real macOS and Windows, verify section/order/icon/title rendering, title/section searches, keyboard and mouse primary actions, paste into another focused application, opening the guide, errors, disposal/relaunch and V1 fallback. Record real artifacts before changing verification states.

Installing this as a V1 GitHub/JavaScript package does not register Native V2 executable dispatch. A standalone `.rlib`, manifest, or `native-v2.json` marker cannot provide an external Rust loader. The existing V1 package and its preferences/storage can remain installed for fallback; this adapter reads or writes no preferences or persistent storage.
