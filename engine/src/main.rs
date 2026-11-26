use std::io::{self, BufRead, Write};

use electrow_engine::json::{self, Value};
use electrow_engine::Engine;

fn parse_error(message: &str) -> String {
    Value::Object(vec![
        ("v".to_string(), Value::Number(1.0)),
        ("id".to_string(), Value::String("invalid".to_string())),
        ("ok".to_string(), Value::Bool(false)),
        (
            "error".to_string(),
            Value::Object(vec![
                (
                    "code".to_string(),
                    Value::String("bad_request".to_string()),
                ),
                ("message".to_string(), Value::String(message.to_string())),
                ("retryable".to_string(), Value::Bool(false)),
            ]),
        ),
    ])
    .to_json()
}

fn main() {
    let mut engine = Engine::new();
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let request = match json::parse(&line) {
            Ok(request) => request,
            Err(error) => {
                if writeln!(stdout, "{}", parse_error(&error)).is_err() {
                    break;
                }
                if stdout.flush().is_err() {
                    break;
                }
                continue;
            }
        };

        let shutdown = request.get("cmd").and_then(Value::as_str) == Some("shutdown");
        let response = engine.handle(&request);
        if writeln!(stdout, "{}", response.to_json()).is_err() {
            break;
        }
        if stdout.flush().is_err() {
            break;
        }
        if shutdown && response.get("ok") == Some(&Value::Bool(true)) {
            break;
        }
    }
}
