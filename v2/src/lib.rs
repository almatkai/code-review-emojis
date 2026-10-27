//! Code Review Emoji Guide, ported from rus64's MIT Raycast extension.
//! All effect data is predefined; renderer payloads carry only issued action IDs.
use serde_json::{json, Value};
use std::fmt::Write;

pub const GUIDE_URL: &str = "https://github.com/erikthedeveloper/code-review-emoji-guide";
pub const EMOJIS: &[(&str, &str)] = &[
    ("😃", "I like this and I want the author to know it!"),
    ("🔧", "I think this needs to be changed."),
    ("❓", "I have a question."),
    ("💭", "Let me think out loud here for a minute."),
    ("🌱", "Planting a seed for future."),
    ("📝", "This is an explanatory note."),
    ("⛏️", "This is a nitpick."),
    ("♻️", "Suggestion for refactoring."),
    (
        "🏕",
        "Here is an opportunity to leave the code cleaner than we found it.",
    ),
    (
        "📌",
        "This is a concern that is out of scope and should be staged appropriately for follow up.",
    ),
];

#[derive(Debug, PartialEq, Eq)]
pub enum Effect {
    Paste(&'static str),
    OpenGuide,
}

#[derive(Debug)]
pub enum Outcome {
    Render { root: Value, actions: Vec<Value> },
    Unchanged,
    Effect(Effect),
}

pub fn commands() -> &'static [&'static str] {
    &["index"]
}

fn generation(state: &Value) -> Result<&str, String> {
    state["generation"]
        .as_str()
        .filter(|s| s.len() == 32 && s.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| "Code Review Emoji session is not active".into())
}

fn action_id(generation: &str, index: usize) -> String {
    format!("code-review-emojis:{generation}:{index}")
}

fn render(state: &Value) -> Result<Outcome, String> {
    let generation = generation(state)?;
    let mut actions = Vec::new();
    let mut rows = Vec::new();
    for (index, (icon, title)) in EMOJIS.iter().enumerate() {
        let id = action_id(generation, index);
        actions.push(json!({"id":id,"title":"Paste","kind":"action"}));
        rows.push(json!({"type":"List.Item","props":{"id":icon,"icon":icon,"title":title,"actionIds":[id]}}));
    }
    let id = action_id(generation, EMOJIS.len());
    actions.push(json!({"id":id,"title":"Open in Browser","kind":"open"}));
    Ok(Outcome::Render {
        root: json!({"type":"List","props":{"searchBarPlaceholder":"Search Emojis"},"children":[
            {"type":"List.Section","props":{"title":"Emojis"},"children":rows},
            {"type":"List.Section","props":{"title":"More"},"children":[
                {"type":"List.Item","props":{"id":"📘","icon":"📘","title":"Open Code Review Emoji Guide","actionIds":[id]}}
            ]}
        ]}),
        actions,
    })
}

/// The host owns and serializes this private per-session state. Never load state
/// from renderer payloads. Launch replaces the generation; disposal invalidates it.
pub fn prepare(
    command: &str,
    operation: &str,
    payload: &Value,
    state: &mut Value,
) -> Result<Outcome, String> {
    if !commands().contains(&command) {
        return Err(format!("Unknown Code Review Emoji command: {command}"));
    }
    match operation {
        "extension:run-command" => {
            // Invalidate the old launch even when OS entropy acquisition fails.
            *state = Value::Null;
            let mut bytes = [0_u8; 16];
            getrandom::getrandom(&mut bytes)
                .map_err(|e| format!("Session entropy unavailable: {e}"))?;
            let mut token = String::with_capacity(32);
            for byte in bytes {
                write!(token, "{byte:02x}").expect("writing to String cannot fail");
            }
            *state = json!({"generation":token});
            render(state)
        }
        "extension:dispose-session" => {
            *state = Value::Null;
            Ok(Outcome::Unchanged)
        }
        "extension:refresh-session" => render(state),
        "extension:search-text-changed" | "extension:load-more" => {
            generation(state)?;
            // The beta renderer filters the complete list on the client.
            Ok(Outcome::Unchanged)
        }
        "extension:invoke-action" => {
            let token = generation(state)?;
            let id = payload["actionId"]
                .as_str()
                .ok_or("Action ID is required")?;
            // Exact membership rejects alternate encodings, indices and generations.
            let index = (0..=EMOJIS.len())
                .find(|&i| id == action_id(token, i))
                .ok_or("Unknown or stale Code Review Emoji action")?;
            Ok(Outcome::Effect(if index == EMOJIS.len() {
                Effect::OpenGuide
            } else {
                Effect::Paste(EMOJIS[index].0)
            }))
        }
        _ => Err(format!(
            "Unsupported Code Review Emoji operation: {operation}"
        )),
    }
}

/// A successful transport is insufficient: the beta platform acknowledges an
/// effect with {"ok":true}. Preserve error envelopes and reject malformed replies.
pub fn acknowledge(reply: Value) -> Result<(), String> {
    if reply.get("error").is_some_and(|e| !e.is_null()) {
        return Err(format!("Native effect failed: {}", reply["error"]));
    }
    if reply["ok"] != true {
        return Err(format!("Native effect was not acknowledged: {reply}"));
    }
    Ok(())
}
