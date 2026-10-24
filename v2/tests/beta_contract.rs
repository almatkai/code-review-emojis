//! The production adapter compiles unchanged with byte-exact beta common/SDK.
//! Only Tauri's unused desktop event surface and the CoreContext router are stubbed.
extern crate self as tauri;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AppHandle;
pub trait Emitter {
    fn emit(&self, channel: &str, payload: Value) -> Result<(), String>;
}
impl Emitter for AppHandle {
    fn emit(&self, _: &str, _: Value) -> Result<(), String> {
        panic!("A headless adapter test must not emit desktop events")
    }
}

#[rustfmt::skip]
#[path = "fixtures/beta/common.rs"]
pub mod beta_common;
#[rustfmt::skip]
#[path = "fixtures/beta/v2_sdk.rs"]
pub mod v2_sdk;
pub mod native_core {
    pub use crate::beta_common as common;
}
#[path = "../src/tezbar_adapter.rs"]
mod adapter;

use beta_common::{CoreContext, CorePaths, CoreResult};
type Calls = Arc<Mutex<Vec<(String, Vec<Value>)>>>;
type Replies = Arc<Mutex<VecDeque<CoreResult>>>;

fn context() -> (tempfile::TempDir, CoreContext, Calls, Replies) {
    let dir = tempfile::tempdir().unwrap();
    let ctx = CoreContext::headless(CorePaths {
        data_dir: dir.path().join("data"),
        config_dir: dir.path().join("config"),
        resource_dir: dir.path().join("resources"),
        temp_dir: dir.path().join("tmp"),
        home_dir: dir.path().into(),
    });
    let calls: Calls = Arc::default();
    let replies: Replies = Arc::default();
    let (seen, queue) = (calls.clone(), replies.clone());
    ctx.set_router(Arc::new(move |channel, args| {
        assert!(matches!(
            channel.as_str(),
            "core:platform:paste-text" | "shell:open"
        ));
        seen.lock().unwrap().push((channel, args));
        let reply = queue
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(Ok(json!({"ok":true})));
        Box::pin(async move { reply })
    }))
    .unwrap();
    (dir, ctx, calls, replies)
}

fn oracle() -> Value {
    serde_json::from_str(include_str!("fixtures/upstream/capture.json")).unwrap()
}
async fn launch(ctx: &CoreContext, state: &mut Value) -> Value {
    adapter::handle(ctx, "index", "extension:run-command", &json!({}), state)
        .await
        .unwrap()
}
async fn invoke(ctx: &CoreContext, state: &mut Value, id: &Value) -> CoreResult {
    adapter::handle(
        ctx,
        "index",
        "extension:invoke-action",
        &json!({
            "actionId":id,
            "text":"ATTACK", "content":"ATTACK", "url":"https://attacker.invalid",
            "state":{"generation":"forged"},
            "formValues":{"text":"ATTACK", "url":"https://attacker.invalid"}
        }),
        state,
    )
    .await
}

#[tokio::test]
async fn every_source_row_and_action_uses_actual_beta_routes() {
    let (_dir, ctx, calls, _) = context();
    let mut state = Value::Null;
    let view = launch(&ctx, &mut state).await;
    assert_eq!(adapter::commands(), &["index"]);
    assert_eq!(view["mode"], "view");
    assert_eq!(view["ok"], true);
    assert_eq!(view["root"]["type"], "List");
    let expected = oracle();
    assert_eq!(
        view["root"]["props"],
        json!({"searchBarPlaceholder": expected["placeholder"]})
    );
    let sections = view["root"]["children"].as_array().unwrap();
    assert_eq!(
        sections.len(),
        expected["sections"].as_array().unwrap().len()
    );
    assert!(calls.lock().unwrap().is_empty());
    let mut action_index = 0;
    for (section, source) in sections
        .iter()
        .zip(expected["sections"].as_array().unwrap())
    {
        assert_eq!(section["props"]["title"], source["title"]);
        let rows = section["children"].as_array().unwrap();
        assert_eq!(rows.len(), source["rows"].as_array().unwrap().len());
        for (row, original) in rows.iter().zip(source["rows"].as_array().unwrap()) {
            assert_eq!(row["props"]["icon"], original["icon"]);
            assert_eq!(row["props"]["title"], original["title"]);
            assert_eq!(row["props"]["actionIds"].as_array().unwrap().len(), 1);
            let id = &row["props"]["actionIds"][0];
            let action = &view["actions"][action_index];
            assert_eq!(action["id"], *id);
            let original_action = &original["actions"][0];
            let (channel, content, title, kind) = match original_action["type"].as_str().unwrap() {
                "Action.Paste" => (
                    "core:platform:paste-text",
                    &original_action["content"],
                    "Paste",
                    "action",
                ),
                "Action.OpenInBrowser" => (
                    "shell:open",
                    &original_action["url"],
                    "Open in Browser",
                    "open",
                ),
                other => panic!("Unported source action {other}"),
            };
            assert_eq!(action["title"], title);
            assert_eq!(action["kind"], kind);
            assert_eq!(
                invoke(&ctx, &mut state, id).await.unwrap(),
                json!({"ok":true,"mode":"unchanged"})
            );
            let seen = calls.lock().unwrap();
            assert_eq!(
                seen.last().unwrap(),
                &(channel.to_owned(), vec![content.clone()])
            );
            action_index += 1;
        }
    }
    assert_eq!(view["actions"].as_array().unwrap().len(), action_index);
    assert_eq!(calls.lock().unwrap().len(), action_index);
    assert_eq!(action_index, 11);
}

#[tokio::test]
async fn failures_never_claim_success_and_explicit_retry_keeps_the_same_action() {
    for index in [0, 10] {
        let (_dir, ctx, calls, replies) = context();
        let mut state = Value::Null;
        let view = launch(&ctx, &mut state).await;
        let id = &view["actions"][index]["id"];
        let failures = [
            Err("transport disconnected".into()),
            Ok(json!({"ok":false,"error":"denied"})),
            Ok(json!({"ok":true,"error":"failed despite flag"})),
            Ok(json!({"error":{"message":"denied"}})),
            Ok(Value::Null),
            Ok(json!({})),
            Ok(json!({"ok":"true"})),
            Ok(json!(false)),
        ];
        let old_state = state.clone();
        for failure in failures {
            let before = calls.lock().unwrap().len();
            replies.lock().unwrap().push_back(failure);
            assert!(invoke(&ctx, &mut state, id).await.is_err());
            assert_eq!(state, old_state);
            // Exactly one request; no implicit duplicate OS effects on failure.
            assert_eq!(calls.lock().unwrap().len(), before + 1);
        }
        assert_eq!(
            invoke(&ctx, &mut state, id).await.unwrap()["mode"],
            "unchanged"
        );
    }
}

#[tokio::test]
async fn invalid_operations_commands_and_fabricated_or_stale_actions_have_no_effects() {
    let (_dir, ctx, calls, _) = context();
    let mut state = Value::Null;
    assert!(invoke(&ctx, &mut state, &json!("code-review-emojis:0"))
        .await
        .is_err());
    let old = launch(&ctx, &mut state).await;
    let id = &old["actions"][0]["id"];
    for command in ["", "INDEX", "other"] {
        for operation in [
            "extension:run-command",
            "extension:invoke-action",
            "extension:dispose-session",
        ] {
            let before = state.clone();
            assert!(adapter::handle(
                &ctx,
                command,
                operation,
                &json!({"actionId":id}),
                &mut state
            )
            .await
            .is_err());
            assert_eq!(state, before);
        }
    }
    for operation in ["", "invoke-action", "paste", "copy", "extension:unknown"] {
        assert!(adapter::handle(
            &ctx,
            "index",
            operation,
            &json!({"actionId":id}),
            &mut state
        )
        .await
        .is_err());
    }
    for fake in [
        Value::Null,
        json!(42),
        json!(""),
        json!("code-review-emojis:forged:0"),
        json!(format!("{}0", id.as_str().unwrap())),
        json!(id.as_str().unwrap().replace(":0", ":99")),
        json!(id.as_str().unwrap().replace(":0", ":00")),
    ] {
        assert!(invoke(&ctx, &mut state, &fake).await.is_err());
    }
    // A distinct active session cannot accept this session's issued ID.
    let mut other_state = Value::Null;
    launch(&ctx, &mut other_state).await;
    assert!(invoke(&ctx, &mut other_state, id).await.is_err());
    let refreshed = adapter::handle(
        &ctx,
        "index",
        "extension:refresh-session",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(old, refreshed);
    let new = launch(&ctx, &mut state).await;
    assert_ne!(old["actions"][0]["id"], new["actions"][0]["id"]);
    assert!(invoke(&ctx, &mut state, id).await.is_err());
    adapter::handle(
        &ctx,
        "index",
        "extension:dispose-session",
        &json!({}),
        &mut state,
    )
    .await
    .unwrap();
    assert_eq!(state, Value::Null);
    assert!(invoke(&ctx, &mut state, &new["actions"][0]["id"])
        .await
        .is_err());
    for operation in [
        "extension:refresh-session",
        "extension:search-text-changed",
        "extension:load-more",
    ] {
        assert!(
            adapter::handle(&ctx, "index", operation, &json!({}), &mut state)
                .await
                .is_err()
        );
    }
    launch(&ctx, &mut state).await;
    assert!(invoke(&ctx, &mut state, &new["actions"][0]["id"])
        .await
        .is_err());
    assert!(calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn client_search_and_load_more_neither_change_actions_nor_trigger_effects() {
    let (_dir, ctx, calls, _) = context();
    let mut state = Value::Null;
    let view = launch(&ctx, &mut state).await;
    for operation in ["extension:search-text-changed", "extension:load-more"] {
        let old = state.clone();
        assert_eq!(
            adapter::handle(
                &ctx,
                "index",
                operation,
                &json!({"searchText":"refactor","actionId":view["actions"][0]["id"]}),
                &mut state
            )
            .await
            .unwrap()["mode"],
            "unchanged"
        );
        assert_eq!(state, old);
    }
    assert!(calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn uninitialized_router_is_a_transport_error() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = CoreContext::headless(CorePaths {
        data_dir: dir.path().into(),
        config_dir: dir.path().into(),
        resource_dir: dir.path().into(),
        temp_dir: dir.path().into(),
        home_dir: dir.path().into(),
    });
    let mut state = Value::Null;
    let view = launch(&ctx, &mut state).await;
    assert!(invoke(&ctx, &mut state, &view["actions"][0]["id"])
        .await
        .unwrap_err()
        .contains("dispatcher is not initialized"));
}
