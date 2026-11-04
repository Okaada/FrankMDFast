//! In-process stand-in for the Rails controllers (`NotesController`,
//! `FoldersController`, `RecentFilesController`) so the controller tests and
//! the fixtures' `"http"` cases can drive the Rust API.
//!
//! This is *test plumbing*: it emulates the Rails routing/HTTP layer that the
//! desktop app does not have, on top of the real API under test
//! ([`NotesService`], [`frankmd_fs::model`], [`RecentFilesService`],
//! `frankmd_core::{path, hugo, tree_view}`). What is emulated (and therefore
//! *not* under test):
//! - routing (`config/routes.rb`), including the `(.:format)` suffix that the
//!   `folders/*path` routes strip from the still-encoded glob (so
//!   `/folders/v1.2` → path `v1`, `/folders/..%2Fx` → path `.`), and the
//!   glob percent-decoding;
//! - `reject_config_file` (403 `{"error":"forbidden"}`) on index/show/create/
//!   update/destroy/rename via `frankmd_core::path::is_config_file`;
//! - status codes and JSON bodies with English i18n messages;
//! - unrescued exceptions → status 500 + `unhandled` (`{error, message}`),
//!   the fixtures' `unhandled_exception`;
//! - HTML responses: bodies are `"<HTML>"` (page) or `"<TREE>"` (tree partial,
//!   with the tree as data in [`HttpResponse::tree`]); `GET /recent_files`
//!   returns the entries as a JSON array instead of the HTML panel;
//! - `hugo_path_style` is read from the `.fed` line directly (Config's job);
//! - the clock is [`super::pinned_now`] in UTC (Rails `Time.current`, TZ unset);
//! - backlinks are sorted by `(path, line_number)` (Rails order is readdir
//!   order; the fixtures sort the same way).
#![allow(dead_code, clippy::disallowed_methods, clippy::unwrap_used)]

use super::{FedRecentStore, Sandbox, en, fs_error_json, pinned_now, sha256_hex};
use frankmd_core::hugo::{HugoPathStyle, NewPost};
use frankmd_core::path::{PathKind, classify, is_config_file, normalize_note_path};
use frankmd_core::tree_view::{TreeView, parse_expanded, remap_expanded, tree_rows};
use frankmd_fs::model::{self, ErrorField, ErrorText, ModelError, SaveMode};
use frankmd_fs::{FsError, NotesService, RecentFilesService, RecentStore};
use frankmd_testkit::serde_json::{Value, json};
use std::collections::BTreeSet;

/// The tree a turbo-stream / tree-partial response would render, as data.
#[derive(Debug, Clone)]
pub(crate) struct TreeRender {
    /// Rows (or the empty state).
    pub(crate) view: TreeView,
    /// Expanded set after any remap (insertion order).
    pub(crate) expanded: Vec<String>,
    /// Selected file.
    pub(crate) selected: String,
}

/// An emulated response.
#[derive(Debug, Clone)]
pub(crate) struct HttpResponse {
    /// HTTP status.
    pub(crate) status: u16,
    /// JSON body (or `"<HTML>"`, `"<TREE>"`, raw asset text, `""` for `head`).
    pub(crate) body: Value,
    /// `Content-Type` for served assets.
    pub(crate) content_type: Option<String>,
    /// The body is an unrescued exception (`{error, message}`).
    pub(crate) unhandled: bool,
    /// Tree rendered by `/notes/tree` and turbo-stream responses.
    pub(crate) tree: Option<TreeRender>,
    /// `X-Created-Path` (turbo-stream note create).
    pub(crate) created_path: Option<String>,
}

impl HttpResponse {
    fn new(status: u16, body: Value) -> Self {
        Self { status, body, content_type: None, unhandled: false, tree: None, created_path: None }
    }
    fn head(status: u16) -> Self {
        Self::new(status, Value::String(String::new()))
    }
    fn unhandled(e: &FsError) -> Self {
        let mut r = Self::new(500, fs_error_json(e));
        r.unhandled = true;
        r
    }

    /// The fixture's result shape: `{status, body}` or
    /// `{status: 500, unhandled_exception: {...}}`.
    pub(crate) fn to_fixture(&self) -> Value {
        if self.unhandled {
            json!({"status": self.status, "unhandled_exception": self.body})
        } else {
            json!({"status": self.status, "body": self.body})
        }
    }

    /// `data-path`s of the rendered tree rows, in document order.
    pub(crate) fn tree_paths(&self) -> Vec<String> {
        match self.tree.as_ref().map(|t| &t.view) {
            Some(TreeView::Rows(rows)) => rows.iter().map(|r| r.path.clone()).collect(),
            _ => vec![],
        }
    }
}

/// The emulated app bound to one sandbox and one recents store.
pub(crate) struct App<'a> {
    /// Sandbox (`NOTES_PATH`).
    pub(crate) sb: &'a Sandbox,
    /// Recents store (`Config`).
    pub(crate) store: &'a dyn RecentStore,
    /// `Time.current`.
    pub(crate) now: jiff::Timestamp,
}

/// One-shot JSON request (`Accept: application/json`) with the default
/// `.fed`-backed recents store.
pub(crate) fn dispatch(sb: &Sandbox, method: &str, url: &str, params: &Value) -> HttpResponse {
    let store = FedRecentStore::new(&sb.root);
    App::new(sb, &store).call(method, url, params, false)
}

/// One-shot turbo-stream request (`Accept: text/vnd.turbo-stream.html`).
pub(crate) fn dispatch_turbo(sb: &Sandbox, method: &str, url: &str, params: &Value) -> HttpResponse {
    let store = FedRecentStore::new(&sb.root);
    App::new(sb, &store).call(method, url, params, true)
}

/// RFC 3986 percent-decoding (bytes, then lossy UTF-8), like Journey's glob unescape.
pub(crate) fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%'
            && i + 2 < b.len()
            && let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16)
        {
            out.push(v);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Strip Rails' optional `(.:format)` from a still-encoded glob
/// (`[^/.?]+` after the last `.`).
fn strip_format(encoded: &str) -> &str {
    match encoded.rfind('.') {
        Some(i) if i + 1 < encoded.len() && !encoded[i + 1..].contains(['/', '.', '?']) => &encoded[..i],
        _ => encoded,
    }
}

fn param<'v>(params: &'v Value, key: &str) -> Option<&'v str> {
    params.get(key).and_then(Value::as_str)
}

fn join_messages(errors: &[ModelError]) -> String {
    errors.iter().map(|e| e.full_message(&|k| en(k))).collect::<Vec<_>>().join(", ")
}

fn has_base_key(errors: &[ModelError], key: &'static str) -> bool {
    errors.iter().any(|e| e.field == ErrorField::Base && e.text == ErrorText::Key(key))
}

fn mime_for(path: &str) -> &'static str {
    let ext = path.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()).unwrap_or_default();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

impl<'a> App<'a> {
    /// Bind to a sandbox and store; the clock is pinned.
    pub(crate) fn new(sb: &'a Sandbox, store: &'a dyn RecentStore) -> Self {
        Self { sb, store, now: pinned_now() }
    }

    /// Route one request. `params` is the Rails params hash (form or query).
    pub(crate) fn call(&self, method: &str, url: &str, params: &Value, turbo: bool) -> HttpResponse {
        let vault = self.sb.vault();
        let svc = NotesService::new(&vault).with_recents(self.store);
        let (path_part, query) = url.split_once('?').unwrap_or((url, ""));
        let mut params = if params.is_object() { params.clone() } else { json!({}) };
        for pair in query.split('&').filter(|p| !p.is_empty()) {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            params[percent_decode(k)] = Value::String(percent_decode(&v.replace('+', " ")));
        }
        let m = method.to_ascii_uppercase();
        let ctx = Ctx { app: self, svc: &svc, params: &params, turbo };

        if path_part == "/" || path_part.is_empty() {
            return ctx.index();
        }
        if path_part == "/recent_files" {
            return ctx.recent_files(&m);
        }
        if path_part == "/notes" && m == "POST" {
            return ctx.guarded(None, |c| c.create(None));
        }
        if path_part == "/notes/tree" && m == "GET" {
            return ctx.tree_partial();
        }
        if path_part == "/notes/search" {
            panic!("/notes/search is frankmd-search's contract (not emulated here)");
        }
        if let Some(rest) = path_part.strip_prefix("/notes/") {
            if m == "POST"
                && let Some(p) = rest.strip_suffix("/rename")
            {
                let p = percent_decode(p);
                return ctx.guarded(Some(&p), |c| c.rename(&p));
            }
            if m == "GET"
                && let Some(p) = rest.strip_suffix("/backlinks")
            {
                return ctx.backlinks(&percent_decode(p));
            }
            let p = percent_decode(rest);
            return match m.as_str() {
                "GET" => ctx.guarded(Some(&p), |c| c.show(&p)),
                "POST" => ctx.guarded(Some(&p), |c| c.create(Some(&p))),
                "PATCH" | "PUT" => ctx.guarded(Some(&p), |c| c.update(&p)),
                "DELETE" => ctx.guarded(Some(&p), |c| c.destroy(&p)),
                _ => HttpResponse::head(404),
            };
        }
        if let Some(rest) = path_part.strip_prefix("/folders/") {
            if m == "POST" {
                if let Some(p) = rest.strip_suffix("/rename") {
                    return ctx.folder_rename(&percent_decode(strip_format(p)));
                }
                return ctx.folder_create(&percent_decode(strip_format(rest)));
            }
            if m == "DELETE" {
                return ctx.folder_destroy(&percent_decode(strip_format(rest)));
            }
        }
        HttpResponse::head(404)
    }
}

struct Ctx<'c, 'a> {
    app: &'c App<'a>,
    svc: &'c NotesService<'c>,
    params: &'c Value,
    turbo: bool,
}

impl Ctx<'_, '_> {
    /// `before_action :reject_config_file` (params file/path/new_path).
    fn guarded(&self, path: Option<&str>, f: impl FnOnce(&Self) -> HttpResponse) -> HttpResponse {
        let candidates = [param(self.params, "file"), path, param(self.params, "new_path")];
        if candidates.into_iter().flatten().any(is_config_file) {
            return HttpResponse::new(403, json!({"error": "forbidden"}));
        }
        f(self)
    }

    fn render_tree(&self, selected: Option<&str>, remap: Option<(&str, &str)>) -> TreeRender {
        let tree = self.svc.list_tree().unwrap();
        let mut expanded = parse_expanded(param(self.params, "expanded").unwrap_or(""));
        if let Some((from, to)) = remap {
            expanded = remap_expanded(&expanded, from, to);
        }
        let selected = selected.map(str::to_owned).unwrap_or_else(|| param(self.params, "selected").unwrap_or("").to_owned());
        let set: BTreeSet<String> = expanded.iter().cloned().collect();
        TreeRender { view: tree_rows(&tree, &set, &selected), expanded, selected }
    }

    fn turbo_ok(&self, status: u16, selected: Option<&str>, remap: Option<(&str, &str)>) -> HttpResponse {
        let mut r = HttpResponse::new(status, Value::String("<TURBO>".into()));
        r.tree = Some(self.render_tree(selected, remap));
        r
    }

    fn index(&self) -> HttpResponse {
        self.guarded(None, |_| HttpResponse::new(200, Value::String("<HTML>".into())))
    }

    fn tree_partial(&self) -> HttpResponse {
        let mut r = HttpResponse::new(200, Value::String("<TREE>".into()));
        r.tree = Some(self.render_tree(None, None));
        r
    }

    fn show(&self, raw: &str) -> HttpResponse {
        if classify(raw) == PathKind::Asset {
            return self.serve_asset(raw);
        }
        let path = normalize_note_path(raw);
        match self.svc.read(&path) {
            Ok(note) => json_ok(json!({
                "path": path,
                "content": String::from_utf8_lossy(&note.bytes),
                "revision": note.revision.0,
            })),
            Err(FsError::NotFound(_)) => HttpResponse::new(404, json!({"error": en("errors.note_not_found")})),
            Err(e) => HttpResponse::unhandled(&e),
        }
    }

    fn serve_asset(&self, raw: &str) -> HttpResponse {
        let root = &self.app.sb.root;
        let Some(full) = frankmd_fs::path_safety::contain(root, Some(raw)) else {
            return HttpResponse::head(403);
        };
        if [frankmd_fs::lock::FILESYSTEM_LOCK, frankmd_fs::lock::CONFIG_LOCK].iter().any(|n| full == root.join(n)) {
            return HttpResponse::head(404);
        }
        match std::fs::metadata(&full) {
            Ok(m) if m.is_file() => {
                let bytes = std::fs::read(&full).unwrap();
                let mut r = HttpResponse::new(200, Value::String(String::from_utf8_lossy(&bytes).into_owned()));
                r.content_type = Some(mime_for(raw).to_owned());
                r
            }
            _ => HttpResponse::head(404),
        }
    }

    fn create(&self, raw: Option<&str>) -> HttpResponse {
        let (path, content): (String, Vec<u8>) = if param(self.params, "template") == Some("hugo") {
            let title = param(self.params, "title").unwrap_or("");
            if title.trim().is_empty() {
                return HttpResponse::new(422, json!({"error": en("errors.title_required")}));
            }
            let parent = param(self.params, "parent").filter(|p| !p.trim().is_empty());
            let template = self.svc.hugo_template().map(|b| String::from_utf8_lossy(&b).into_owned());
            let post = frankmd_core::hugo::generate_post(&NewPost {
                title,
                parent,
                now: self.app.now.to_zoned(jiff::tz::TimeZone::UTC),
                style: self.hugo_style(),
                template: template.as_deref(),
            });
            (post.path, post.content.into_bytes())
        } else {
            (normalize_note_path(raw.unwrap_or("")), param(self.params, "content").unwrap_or("").as_bytes().to_vec())
        };
        match model::note_exists(self.svc, &path) {
            Ok(true) => return HttpResponse::new(422, json!({"error": en("errors.note_already_exists")})),
            Ok(false) => {}
            Err(e) => return HttpResponse::unhandled(&e),
        }
        match model::note_save(self.svc, &path, Some(&content), SaveMode::CreateOnly) {
            Ok(Ok(saved)) => {
                if self.turbo {
                    let mut r = self.turbo_ok(201, Some(&saved), None);
                    r.created_path = Some(saved);
                    r
                } else {
                    HttpResponse::new(201, json!({"path": saved, "message": en("success.note_created")}))
                }
            }
            Ok(Err(errors)) => HttpResponse::new(422, json!({"error": join_messages(&errors)})),
            Err(e) => HttpResponse::unhandled(&e),
        }
    }

    /// `Config.new.get("hugo_path_style")` from the `.fed` line (ENV ignored).
    fn hugo_style(&self) -> HugoPathStyle {
        let text = std::fs::read_to_string(self.app.sb.root.join(".fed")).unwrap_or_default();
        let value = text.lines().find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == "hugo_path_style").then(|| v.trim().trim_matches(['"', '\'']).to_owned())
        });
        HugoPathStyle::from_config(value.as_deref())
    }

    fn update(&self, raw: &str) -> HttpResponse {
        let path = normalize_note_path(raw);
        let content = param(self.params, "content").unwrap_or("").as_bytes().to_vec();
        match model::note_save(self.svc, &path, Some(&content), SaveMode::ExistingOnly) {
            Ok(Ok(saved)) => json_ok(json!({
                "path": saved,
                "message": en("success.note_saved"),
                "revision": sha256_hex(&content),
            })),
            Ok(Err(errors)) => {
                let status = if has_base_key(&errors, "errors.note_not_found") { 404 } else { 422 };
                HttpResponse::new(status, json!({"error": join_messages(&errors)}))
            }
            Err(e) => HttpResponse::unhandled(&e),
        }
    }

    fn destroy(&self, raw: &str) -> HttpResponse {
        let path = normalize_note_path(raw);
        match model::note_destroy(self.svc, &path, param(self.params, "expected_revision")) {
            Ok(Ok(())) => {
                if self.turbo {
                    self.turbo_ok(200, None, None)
                } else {
                    json_ok(json!({"message": en("success.note_deleted")}))
                }
            }
            Ok(Err(errors)) if errors.iter().any(|e| e.revision_conflict) => {
                HttpResponse::new(409, json!({"error": join_messages(&errors), "code": "revision_conflict"}))
            }
            Ok(Err(errors)) => HttpResponse::new(404, json!({"error": join_messages(&errors)})),
            Err(e) => HttpResponse::unhandled(&e),
        }
    }

    fn rename(&self, raw: &str) -> HttpResponse {
        let path = normalize_note_path(raw);
        match model::note_exists(self.svc, &path) {
            Ok(true) => {}
            Ok(false) => return HttpResponse::new(404, json!({"error": en("errors.note_not_found")})),
            Err(e) => return HttpResponse::unhandled(&e),
        }
        let new_path = normalize_note_path(param(self.params, "new_path").unwrap_or(""));
        match model::note_rename(self.svc, &path, &new_path) {
            Ok(Ok(renamed)) => {
                if self.turbo {
                    self.turbo_ok(200, Some(&renamed), None)
                } else {
                    json_ok(json!({"old_path": path, "new_path": renamed, "message": en("success.note_renamed")}))
                }
            }
            Ok(Err(errors)) => HttpResponse::new(422, json!({"error": join_messages(&errors)})),
            Err(e) => HttpResponse::unhandled(&e),
        }
    }

    fn backlinks(&self, raw: &str) -> HttpResponse {
        let path = normalize_note_path(raw);
        match self.svc.backlinks(&path, 3) {
            Ok(mut links) => {
                links.sort_by(|a, b| (&a.path, a.line_number).cmp(&(&b.path, b.line_number)));
                let count = links.len();
                json_ok(json!({"backlinks": links, "count": count}))
            }
            Err(e) => HttpResponse::unhandled(&e),
        }
    }

    fn folder_create(&self, path: &str) -> HttpResponse {
        match model::folder_exists(self.svc, path) {
            Ok(true) => return HttpResponse::new(422, json!({"error": en("errors.folder_already_exists")})),
            Ok(false) => {}
            Err(e) => return HttpResponse::unhandled(&e),
        }
        match model::folder_create(self.svc, path) {
            Ok(Ok(())) => {
                if self.turbo {
                    self.turbo_ok(201, None, None)
                } else {
                    HttpResponse::new(201, json!({"path": path, "message": en("success.folder_created")}))
                }
            }
            Ok(Err(errors)) => HttpResponse::new(422, json!({"error": join_messages(&errors)})),
            Err(e) => HttpResponse::unhandled(&e),
        }
    }

    fn folder_error(errors: &[ModelError]) -> HttpResponse {
        let msg = join_messages(errors);
        let status = if msg.contains("not found") { 404 } else { 422 };
        HttpResponse::new(status, json!({"error": msg}))
    }

    fn folder_destroy(&self, path: &str) -> HttpResponse {
        match model::folder_destroy(self.svc, path) {
            Ok(Ok(())) => {
                if self.turbo {
                    self.turbo_ok(200, None, None)
                } else {
                    json_ok(json!({"message": en("success.folder_deleted")}))
                }
            }
            Ok(Err(errors)) => Self::folder_error(&errors),
            Err(e) => HttpResponse::unhandled(&e),
        }
    }

    fn folder_rename(&self, path: &str) -> HttpResponse {
        let new_path = param(self.params, "new_path").unwrap_or("").to_owned();
        match model::folder_rename(self.svc, path, &new_path) {
            Ok(Ok(renamed)) => {
                if self.turbo {
                    self.turbo_ok(200, Some(&renamed), Some((path, &renamed)))
                } else {
                    json_ok(json!({"old_path": path, "new_path": renamed, "message": en("success.folder_renamed")}))
                }
            }
            Ok(Err(errors)) => Self::folder_error(&errors),
            Err(e) => HttpResponse::unhandled(&e),
        }
    }

    fn recent_files(&self, method: &str) -> HttpResponse {
        let vault = self.svc.vault();
        let history = RecentFilesService::new(vault, self.app.store);
        match method {
            "GET" => {
                let entries = history.entries();
                HttpResponse::new(200, frankmd_testkit::serde_json::to_value(entries).unwrap())
            }
            "POST" => match history.record(param(self.params, "path"), self.app.now) {
                Ok(true) => HttpResponse::head(204),
                Ok(false) => HttpResponse::head(503),
                Err(FsError::NotFound(_)) => HttpResponse::head(404),
                Err(e) => HttpResponse::unhandled(&e),
            },
            "DELETE" => match history.clear() {
                Ok(true) => HttpResponse::head(204),
                Ok(false) => HttpResponse::head(503),
                Err(e) => HttpResponse::unhandled(&e),
            },
            _ => HttpResponse::head(404),
        }
    }
}

fn json_ok(body: Value) -> HttpResponse {
    HttpResponse::new(200, body)
}
