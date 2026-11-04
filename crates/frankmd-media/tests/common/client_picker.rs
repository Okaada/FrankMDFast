//! Fakes for the picker-source ports: a FIFO transport that records requests in
//! the golden fixtures' encoding, and the three `window.t` modes.
#![allow(dead_code, unreachable_pub, clippy::unwrap_used)]

use std::collections::VecDeque;

use frankmd_media::picker::{
    FormField, JsArg, PickedFile, PickerRequest, PickerResponse, RequestBody, TextOptions, Transport, Translator,
};
use frankmd_testkit::serde_json::{self, Map, Value, json};

/// Canned replies served FIFO (`{ok,status,json}` | `{reject}` | `{abort}`).
#[derive(Debug, Default)]
pub struct FakeTransport {
    pub replies: VecDeque<PickerResponse>,
    pub requests: Vec<PickerRequest>,
}

impl FakeTransport {
    pub fn push_json(&mut self, json: Value) {
        self.replies.push_back(PickerResponse::Reply { ok: true, status: 200, json });
    }

    pub fn push_fixture(&mut self, r: &Value) {
        if r.get("abort").is_some() {
            self.replies.push_back(PickerResponse::Aborted);
        } else if let Some(msg) = r.get("reject") {
            self.replies.push_back(PickerResponse::NetworkError(msg.as_str().unwrap_or("").into()));
        } else {
            let ok = r.get("ok").and_then(Value::as_bool).unwrap_or(true);
            let status = r.get("status").and_then(Value::as_u64).map_or(if ok { 200 } else { 500 }, |s| s as u16);
            self.replies.push_back(PickerResponse::Reply { ok, status, json: r.get("json").cloned().unwrap_or(Value::Null) });
        }
    }

    /// Requests in the fixture encoding, draining the log.
    pub fn take_requests(&mut self) -> Value {
        Value::Array(self.requests.drain(..).map(|r| request_json(&r)).collect())
    }
}

impl Transport for FakeTransport {
    fn send(&mut self, request: PickerRequest) -> PickerResponse {
        self.requests.push(request);
        self.replies.pop_front().unwrap_or(PickerResponse::NetworkError("no canned response".into()))
    }
}

pub fn request_json(r: &PickerRequest) -> Value {
    let body = match &r.body {
        RequestBody::None => Value::Null,
        RequestBody::Json(v) => v.clone(),
        RequestBody::Form(fields) => json!({"$formData": fields.iter().map(|(k, f)| json!([k, match f {
            FormField::Text(s) => Value::String(s.clone()),
            FormField::File { name, size, mime } => json!({"$file": name, "size": size, "type": mime}),
        }])).collect::<Vec<_>>()}),
    };
    json!({"method": r.method, "url": r.url, "responseKind": "json", "body": body,
           "signal": if r.abortable { Value::Bool(true) } else { Value::Null }})
}

/// `window.t` returns the key → treated as missing.
pub struct Identity;

impl Translator for Identity {
    fn t(&self, full_key: &str, _options: &TextOptions) -> String {
        full_key.to_owned()
    }
}

/// `window.t` returns `"[pt] <key> <JSON.stringify(options)>"` (options part omitted when `{}`).
pub struct Pt;

impl Translator for Pt {
    fn t(&self, full_key: &str, options: &TextOptions) -> String {
        let mut m = Map::new();
        for (k, v) in options {
            let jv = match v {
                JsArg::Undefined => continue,
                JsArg::Null => Value::Null,
                JsArg::Bool(b) => Value::Bool(*b),
                JsArg::Number(n) if n.is_finite() && n.fract() == 0.0 => json!(*n as i64),
                JsArg::Number(n) => serde_json::Number::from_f64(*n).map_or(Value::Null, Value::Number),
                JsArg::Str(s) => Value::String(s.clone()),
            };
            m.insert(k.clone(), jv);
        }
        if m.is_empty() { format!("[pt] {full_key}") } else { format!("[pt] {full_key} {}", Value::Object(m)) }
    }
}

pub fn translator(mode: &str) -> Option<&'static dyn Translator> {
    match mode {
        "identity" => Some(&Identity),
        "pt" => Some(&Pt),
        _ => None,
    }
}

/// JS-value argument (`{"$undefined":true}` → Undefined; absent → Undefined).
pub fn js_arg(v: Option<&Value>) -> JsArg {
    match v {
        None => JsArg::Undefined,
        Some(Value::Object(o)) if o.contains_key("$undefined") => JsArg::Undefined,
        Some(v) => JsArg::from_json(v),
    }
}

fn mime_for(name: &str) -> String {
    let lower = name.to_lowercase();
    let ext = lower.rsplit('.').next().unwrap_or("");
    match ext {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "svg" => "image/svg+xml",
        _ => "",
    }
    .to_owned()
}

/// Fixture file spec `{name, lastModified=0, size=1024, width?, height?}`
/// (no width → the image fails to decode → null dimensions).
pub fn picked_file(v: &Value) -> PickedFile {
    let name = v["name"].as_str().unwrap_or_default().to_owned();
    let dims = match (v.get("width").and_then(Value::as_u64), v.get("height").and_then(Value::as_u64)) {
        (Some(w), Some(h)) => Some((w as u32, h as u32)),
        _ => None,
    };
    PickedFile {
        mime: mime_for(&name),
        last_modified: v.get("lastModified").and_then(Value::as_i64).unwrap_or(0),
        size: v.get("size").and_then(Value::as_u64).unwrap_or(1024),
        dimensions: dims,
        name,
    }
}

pub fn file(name: &str, last_modified: i64, size: u64) -> PickedFile {
    PickedFile { name: name.into(), last_modified, size, mime: mime_for(name), dimensions: None }
}

/// `window.t` backed by a closure-like table: returns `table[key]` (with
/// `%{count}` replaced) or the key itself.
pub struct Table(pub Vec<(&'static str, &'static str)>);

impl Translator for Table {
    fn t(&self, full_key: &str, options: &TextOptions) -> String {
        match self.0.iter().find(|(k, _)| *k == full_key) {
            Some((_, v)) => {
                let mut s = (*v).to_owned();
                for (k, a) in options {
                    let val = match a {
                        JsArg::Number(n) if n.fract() == 0.0 => format!("{}", *n as i64),
                        JsArg::Number(n) => format!("{n}"),
                        JsArg::Str(x) => x.clone(),
                        other => format!("{other:?}"),
                    };
                    s = s.replace(&format!("%{{{k}}}"), &val);
                }
                s
            }
            None => full_key.to_owned(),
        }
    }
}

pub fn s(v: &str) -> JsArg {
    JsArg::Str(v.to_owned())
}
