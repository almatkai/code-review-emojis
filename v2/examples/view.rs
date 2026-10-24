use code_review_emojis::{prepare, Outcome};
use serde_json::{json, Value};

fn main() {
    let mut state = Value::Null;
    let Outcome::Render { root, actions } =
        prepare("index", "extension:run-command", &json!({}), &mut state).unwrap()
    else {
        panic!("launch must render")
    };
    println!("{}", json!({"root":root,"actions":actions}));
}
