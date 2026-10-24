// Compiled-in adapter for Tezbar beta fd300e276356587726566026f072563eb9c34cb1.
// Include this file in native_core::extensions; see INTEGRATION.md.
use super::v2_sdk;
use crate::native_core::common::{CoreContext, CoreResult};
use code_review_emojis::{acknowledge, prepare, Effect, Outcome, GUIDE_URL};
use serde_json::{json, Value};

pub fn commands() -> &'static [&'static str] {
    code_review_emojis::commands()
}

pub async fn handle(
    ctx: &CoreContext,
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> CoreResult {
    match prepare(command, operation, payload, state)? {
        Outcome::Render { root, actions } => {
            Ok(v2_sdk::view("Code Review Emoji Guide", root, actions))
        }
        Outcome::Unchanged => Ok(v2_sdk::unchanged()),
        Outcome::Effect(effect) => {
            let reply = match effect {
                Effect::Paste(text) => {
                    // This beta route writes the clipboard, hides Tezbar, restores
                    // focus and pastes. Do not repeat these steps in the adapter.
                    ctx.request("core:platform:paste-text", vec![json!(text)])
                        .await?
                }
                Effect::OpenGuide => v2_sdk::open(ctx, GUIDE_URL).await?,
            };
            acknowledge(reply)?;
            // Keep the view/session usable for another selection or explicit retry.
            // No automatic retry: transport failure may occur after an OS effect.
            Ok(v2_sdk::unchanged())
        }
    }
}
