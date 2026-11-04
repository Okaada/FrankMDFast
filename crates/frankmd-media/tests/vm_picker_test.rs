//! Insert image dialog view model. Adapted rows of docs/testing/ui-test-plan.md
//! (image_sources/*.test.js, drop_images, local_images, folder_images,
//! image_picker controller tests).
#![allow(clippy::unwrap_used)]

use std::path::PathBuf;
use std::sync::Arc;

use frankmd_media::library::{PickerCollection, PickerImage, PickerPage};
use frankmd_media::search::{SearchResults, WebImage};
use frankmd_media::vm::Text;
use frankmd_media::vm::picker::*;
use frankmd_media::vm::storage::Provider;

fn config() -> PickerConfig {
    PickerConfig {
        notes_root: PathBuf::from("/notes"),
        images_path: Some(PathBuf::from("/srv/images")),
        server_images: true,
        google_enabled: true,
        ai_enabled: true,
        ai_model: "google/gemini".into(),
        image_extensions: Vec::new(),
    }
}

fn today() -> jiff::civil::Date {
    jiff::civil::date(2026, 7, 4)
}

fn picker() -> ImagePicker {
    let mut p = ImagePicker::default();
    p.open(config(), Some("blog/post.md".into()), Provider::Local, true, today());
    p
}

fn file(name: &str, modified_ms: i64) -> LocalFile {
    LocalFile {
        name: name.into(),
        path: Some(PathBuf::from(format!("/tmp/{name}"))),
        bytes: None,
        size: 102_400,
        modified_ms,
        dims: Some((800, 600)),
    }
}

fn web(url: &str, title: Option<&str>) -> WebImage {
    WebImage {
        url: url.into(),
        thumbnail: None,
        title: title.map(str::to_owned),
        source: Some("example.com".into()),
        width: Some(800),
        height: Some(600),
    }
}

fn results(images: Vec<WebImage>) -> SearchResults {
    SearchResults { images, ..SearchResults::default() }
}

fn search(p: &mut ImagePicker, engine: Engine, images: Vec<WebImage>) {
    p.set_query(engine, "cats");
    let effects = p.search(engine);
    let [PickerEffect::Search { generation, start, .. }] = effects.as_slice() else { panic!("{effects:?}") };
    p.search_result(engine, *generation, *start, Ok(results(images)));
}

fn page(collection: PickerCollection, names: &[&str], total: usize, offset: usize) -> PickerPage {
    PickerPage {
        images: names
            .iter()
            .map(|n| PickerImage {
                name: (*n).into(),
                path: match collection {
                    PickerCollection::Managed => format!("images/{n}"),
                    PickerCollection::Server => format!("photos/{n}"),
                },
                url: match collection {
                    PickerCollection::Managed => format!("images/{n}"),
                    PickerCollection::Server => format!("/images/preview/photos/{n}"),
                },
                preview_url: String::new(),
                item: None,
            })
            .collect(),
        collection,
        total,
        limit: 10,
        offset,
        server_available: true,
    }
}

fn load_library(p: &mut ImagePicker, collection: PickerCollection, names: &[&str]) {
    let effects = p.select_source(Source::Library);
    let gen_ = match effects.as_slice() {
        [PickerEffect::LoadLibrary { generation, .. }] => *generation,
        _ => panic!("{effects:?}"),
    };
    let effects = if collection == PickerCollection::Server { p.set_collection(collection) } else { Vec::new() };
    let gen_ = match effects.as_slice() {
        [PickerEffect::LoadLibrary { generation, .. }] => *generation,
        _ => gen_,
    };
    p.library_result(gen_, Ok(page(collection, names, names.len(), 0)));
}

// adapts: drop_images_controller.test.js rows 1-2
#[test]
fn ingest_replaces_grid_and_reports_deselection_only_when_selected() {
    let mut p = picker();
    assert!(!p.ingest(vec![file("a.png", 1)]));
    p.select_card("upload-0");
    assert_eq!(p.selected_name(), Some("a.png"));
    assert!(p.ingest(vec![file("b.png", 2)]));
    assert_eq!(p.selected_name(), None);
    assert!(!p.can_insert());
}

// adapts: drop_images row 6, image_picker_controller.test.js:59 (paste pre-selects)
#[test]
fn paste_opens_on_upload_preselected() {
    let mut p = ImagePicker::default();
    let pasted = LocalFile {
        name: "pasted-image-1730000000000.png".into(),
        path: None,
        bytes: Some(Arc::new(vec![1, 2, 3])),
        size: 3,
        modified_ms: 1_730_000_000_000,
        dims: None,
    };
    p.open_with_files(config(), Some("n.md".into()), Provider::Local, true, today(), vec![pasted]);
    assert_eq!(p.source(), Source::Upload);
    assert_eq!(p.selected_name(), Some("pasted-image-1730000000000.png"));
    assert_eq!(p.alt(), "pasted image 1730000000000");
    assert_eq!(p.busy(), None, "processing never sticks");
    assert!(matches!(p.upload_cards()[0].thumb, Some(Thumb::Bytes(_))));
}

// adapts: drop_images row 7, FE-318 (non-images refused with a reason)
#[test]
fn rejected_files_explain_why() {
    let mut p = picker();
    p.ingest_external(vec![file("notes.txt", 1), file("README", 2)]);
    assert_eq!(p.selected_name(), None);
    let rejected = p.upload_rejections();
    assert_eq!(rejected.len(), 2);
    let Text::Key(key, args) = &rejected[0] else { panic!() };
    assert_eq!(*key, "dialogs.image_picker.drop_rejected");
    assert_eq!(args[0], ("name", Text::Raw("notes.txt".into())));
    assert_eq!(args[1], ("ext", Text::Raw(".txt".into())));
    assert_eq!(args[2].1, Text::Raw(".jpg, .jpeg, .png, .gif, .webp, .bmp".into()));
    let Text::Key(_, args) = &rejected[1] else { panic!() };
    assert_eq!(args[1], ("ext", Text::key("dialogs.image_picker.no_file_extension")));
}

// FE-319: newest first, ten shown, status texts; folder rows 9-11.
#[test]
fn upload_grid_newest_first_ten_max_with_status() {
    let mut p = picker();
    let files: Vec<LocalFile> = (0..12).map(|i| file(&format!("f{i}.png"), i)).collect();
    p.ingest(files);
    let cards = p.upload_cards();
    assert_eq!(cards.len(), 10);
    assert_eq!(cards[0].label, "f11.png");
    assert_eq!(cards[0].badge.as_deref(), Some("800x600"));
    assert_eq!(cards[0].tooltip, "f11.png (800x600) - 100 KB");
    assert_eq!(
        p.upload_status(),
        Some(Text::with("dialogs.image_picker.folder_showing_recent", &[("shown", "10"), ("total", "12")]))
    );
    p.ingest(vec![file("a.png", 1), file("b.png", 2)]);
    assert_eq!(p.upload_status(), Some(Text::with("dialogs.image_picker.folder_images_found_many", &[("count", "2")])));
    p.ingest(vec![file("a.png", 1)]);
    assert_eq!(p.upload_status(), Some(Text::key("dialogs.image_picker.folder_images_found_one")));
}

// adapts: image_picker_controller.test.js:124 + FE-311/312 (insert flow)
#[test]
fn upload_insert_stores_then_inserts_note_relative_markdown() {
    let mut p = picker();
    p.ingest_external(vec![file("photo.png", 1)]);
    let effects = p.insert(true);
    let [PickerEffect::Store(StoreRequest::Upload { file, resize, s3_key })] = effects.as_slice() else { panic!("{effects:?}") };
    assert_eq!(file.name, "photo.png");
    assert_eq!((*resize, s3_key.clone()), (None, None), "local: no resize, no key");
    assert_eq!(p.busy(), Some(Busy::Insert));
    assert!(!p.can_insert());
    let effects = p.store_result(Ok("images/20260704_101500_ab12cd34_photo.png".into()));
    let [PickerEffect::Insert(result)] = effects.as_slice() else { panic!("{effects:?}") };
    assert_eq!(result.markdown, "![photo](../images/20260704_101500_ab12cd34_photo.png)");
}

// adapts: image_picker_controller.test.js:104 "keeps insert failures inline" (FE-314)
#[test]
fn insert_failure_stays_inline_and_refreshes_s3_storage() {
    let mut p = ImagePicker::default();
    p.open(config(), None, Provider::S3, true, today());
    p.ingest_external(vec![file("photo.png", 1)]);
    let effects = p.insert(true);
    let [PickerEffect::Store(StoreRequest::Upload { resize, s3_key, .. })] = effects.as_slice() else { panic!() };
    assert_eq!(*resize, Some(0.5));
    assert_eq!(s3_key.as_deref(), Some("frankmd/2026/07/photo.png"));
    let effects = p.store_result(Err(StoreError { message: "boom".into(), fallback: false }));
    assert_eq!(effects, vec![PickerEffect::RefreshStorage]);
    assert_eq!(
        p.error(),
        Some(&Text::Key("dialogs.image_picker.insert_failed", vec![("error", Text::Raw("boom".into()))]))
    );
    assert!(p.can_insert(), "Insert re-enabled");
}

// adapts: image_picker_controller.test.js:147 "waits for a provider save before uploading"
#[test]
fn insert_waits_for_a_pending_provider_save() {
    let mut p = picker();
    p.ingest_external(vec![file("photo.png", 1)]);
    assert!(p.insert(false).is_empty());
    assert!(p.waiting_for_storage());
    let effects = p.resume_insert(true);
    assert_eq!(effects.len(), 1);
    assert!(matches!(effects[0], PickerEffect::Store(_)));
    assert!(p.resume_insert(true).is_empty(), "exactly one upload");
    let mut q = picker();
    q.ingest_external(vec![file("photo.png", 1)]);
    q.insert(false);
    assert!(q.resume_insert(false).is_empty());
    assert!(q.error().is_some());
}

// adapts: image_picker_controller.test.js:75 (sources keyboard-navigable; wrap)
#[test]
fn sources_step_with_wrap_and_library_loads_page_one() {
    let mut p = picker();
    let effects = p.step_source(true);
    assert_eq!(p.source(), Source::Library);
    assert!(matches!(
        effects.as_slice(),
        [PickerEffect::LoadLibrary { collection: PickerCollection::Managed, limit: 10, offset: 0, .. }]
    ));
    p.select_source(Source::Upload);
    p.step_source(false);
    assert_eq!(p.source(), Source::Ai);
    assert_eq!(Source::ALL.len(), 7);
}

// adapts: local_images rows 1, 3, 4, 8; local_images_controller rows 2-3
#[test]
fn library_cards_selection_and_existing_references() {
    let mut p = picker();
    load_library(&mut p, PickerCollection::Managed, &["cat.jpg", "dog.jpg"]);
    let cards = p.library_cards();
    assert_eq!(cards[0].label, "cat.jpg");
    assert_eq!(cards[0].thumb, Some(Thumb::File(PathBuf::from("/notes/images/cat.jpg"))));
    assert_eq!(cards[0].tooltip, "cat.jpg");
    p.select_card("images/cat.jpg");
    let cards = p.library_cards();
    assert!(cards[0].selected && !cards[1].selected);
    assert!(!p.storage_row_visible(), "note images are referenced, not copied");
    let effects = p.insert(true);
    let [PickerEffect::Insert(result)] = effects.as_slice() else { panic!("{effects:?}") };
    assert_eq!(result.markdown, "![cat](../images/cat.jpg)");
}

// adapts: local_images_controller rows 2, 4 and FE-325 (image folder copy to S3)
#[test]
fn server_image_hotlinks_or_imports_with_key() {
    let mut p = ImagePicker::default();
    p.open(config(), Some("n.md".into()), Provider::S3, true, today());
    load_library(&mut p, PickerCollection::Server, &["photo.png"]);
    assert_eq!(p.library_cards()[0].thumb, Some(Thumb::File(PathBuf::from("/srv/images/photos/photo.png"))));
    p.select_card("photos/photo.png");
    let effects = p.insert(true);
    let [PickerEffect::Insert(result)] = effects.as_slice() else { panic!() };
    assert_eq!(result.markdown, "![photo](/images/preview/photos/photo.png)");
    p.select_card("photos/photo.png");
    p.row_mut().set_save_copy(true);
    p.row_mut().on_key_edited("folder/photo.png");
    let effects = p.insert(true);
    assert_eq!(
        effects,
        vec![PickerEffect::Store(StoreRequest::ImportPath {
            path: "photos/photo.png".into(),
            resize: Some(0.5),
            s3_key: Some("folder/photo.png".into()),
        })]
    );
}

// adapts: local_images_controller rows 2 (collection/page reset), 9 (folder unavailable)
#[test]
fn library_paging_and_collection_rules() {
    let mut p = picker();
    load_library(&mut p, PickerCollection::Managed, &["a.png"]);
    p.select_card("images/a.png");
    let effects = p.set_library_per_page(25);
    assert!(matches!(effects.as_slice(), [PickerEffect::LoadLibrary { limit: 25, offset: 0, .. }]));
    assert_eq!(p.selected_name(), None);
    let PickerEffect::LoadLibrary { generation, .. } = effects[0].clone() else { panic!() };
    p.library_result(generation, Ok(PickerPage { limit: 25, ..page(PickerCollection::Managed, &["a.png"], 60, 0) }));
    let effects = p.library_page(true);
    assert!(matches!(effects.as_slice(), [PickerEffect::LoadLibrary { offset: 25, .. }]));
    let mut no_server = ImagePicker::default();
    no_server.open(PickerConfig { server_images: false, ..config() }, None, Provider::Local, false, today());
    assert!(no_server.set_collection(PickerCollection::Server).is_empty());
    assert_eq!(no_server.collection(), PickerCollection::Managed);
    assert!(no_server.server_collection_disabled_reason().is_some());
}

// adapts: local_images_controller rows 5-8 (delete via the right collection, confirm, errors)
#[test]
fn library_delete_confirms_and_steps_back_a_page() {
    let mut p = picker();
    load_library(&mut p, PickerCollection::Managed, &["photo.png"]);
    let text = p.request_delete("images/photo.png").unwrap();
    assert_eq!(text, Text::with("dialogs.image_picker.delete_confirm", &[("name", "photo.png")]));
    assert!(p.confirm_delete(false).is_empty(), "cancel deletes nothing");
    p.request_delete("images/photo.png");
    assert_eq!(
        p.confirm_delete(true),
        vec![PickerEffect::DeleteImage { collection: PickerCollection::Managed, path: "images/photo.png".into() }]
    );
    let effects = p.delete_result(Err(Text::Raw("delete failed".into())));
    assert!(effects.is_empty());
    assert_eq!(p.library_error(), Some(&Text::Raw("delete failed".into())));
    let effects = p.delete_result(Ok(()));
    let PickerEffect::LoadLibrary { generation, .. } = effects[0].clone() else { panic!() };
    p.library_result(generation, Ok(page(PickerCollection::Managed, &[], 0, 0)));
    assert_eq!(p.library_error(), None, "cleared after a successful load");
}

#[test]
fn stale_library_results_are_dropped() {
    let mut p = picker();
    let first = p.select_source(Source::Library);
    let second = p.set_library_search("cat");
    let (PickerEffect::LoadLibrary { generation: g1, .. }, PickerEffect::LoadLibrary { generation: g2, .. }) =
        (first[0].clone(), second[0].clone())
    else {
        panic!()
    };
    p.library_result(g1, Ok(page(PickerCollection::Managed, &["old.png"], 1, 0)));
    assert!(p.library_cards().is_empty());
    p.library_result(g2, Ok(page(PickerCollection::Managed, &["cat.png"], 1, 0)));
    assert_eq!(p.library_cards()[0].label, "cat.png");
}

// adapts: web_images rows 1-9, google rows 1-5, pinterest rows 1-6
#[test]
fn search_cards_titles_badges_and_unsafe_thumbnails() {
    let mut p = picker();
    p.select_source(Source::Web);
    let mut with_thumb = web("https://x/full.jpg", Some("<script>alert('xss')</script>"));
    with_thumb.thumbnail = Some("https://x/thumb.jpg".into());
    search(&mut p, Engine::Web, vec![with_thumb, web("javascript:alert(1)", None), web("//evil/x.png", Some("t"))]);
    let cards = p.search_cards(Engine::Web);
    assert_eq!(cards[0].label, "<script>alert('xss')</script>", "shown literally");
    assert_eq!(cards[0].thumb, Some(Thumb::Url("https://x/thumb.jpg".into())));
    assert_eq!(cards[0].caption.as_deref(), Some("example.com"));
    assert_eq!(cards[0].badge.as_deref(), Some("800x600"));
    assert_eq!(cards[1].thumb, None, "javascript: never fetched");
    assert_eq!(cards[1].tooltip, "Image");
    assert_eq!(cards[2].thumb, None, "protocol-relative never fetched");
    assert_eq!(
        p.search_status(Engine::Web).0,
        Some(&Text::with("dialogs.image_picker.search_result_many", &[("count", "3")]))
    );
    p.select_card("0");
    assert_eq!(p.selected_name(), Some("<script>alert('xss')</script>"));
    let effects = p.insert(true);
    let [PickerEffect::Insert(r)] = effects.as_slice() else { panic!() };
    assert_eq!(r.url, "https://x/full.jpg", "hotlinked by default (insert keeps the full URL)");
}

#[test]
fn search_selection_clears_on_new_search_and_broken_thumbs_go() {
    let mut p = picker();
    p.select_source(Source::Pinterest);
    search(&mut p, Engine::Pinterest, vec![web("https://p/1.jpg", Some("DIY-Project_x")), web("https://p/2.jpg", None)]);
    p.select_card("0");
    assert_eq!(p.alt(), "DIY Project x");
    p.thumbnail_failed(Engine::Pinterest, "1");
    assert_eq!(p.search_cards(Engine::Pinterest).len(), 1);
    assert_eq!(p.selected_name(), Some("DIY-Project_x"));
    p.thumbnail_failed(Engine::Pinterest, "0");
    assert_eq!(p.selected_name(), None, "the selected card went away");
    search(&mut p, Engine::Pinterest, vec![web("https://p/3.jpg", None)]);
    p.select_card("0");
    assert_eq!(p.alt(), "", "missing title → empty alt");
    search(&mut p, Engine::Pinterest, vec![web("https://p/4.jpg", None)]);
    assert_eq!(p.selected_name(), None);
}

#[test]
fn search_status_messages() {
    let mut p = picker();
    p.select_source(Source::Web);
    assert!(p.search(Engine::Web).is_empty());
    assert_eq!(p.search_status(Engine::Web).0, Some(&Text::key("dialogs.image_picker.search_keywords_required")));
    p.set_query(Engine::Web, "x");
    let effects = p.search(Engine::Web);
    assert_eq!(p.search_status(Engine::Web).0, Some(&Text::key("status.searching")));
    let PickerEffect::Search { generation, .. } = effects[0].clone() else { panic!() };
    p.search_result(
        Engine::Web,
        generation,
        1,
        Ok(SearchResults { note: Some("Try other keywords".into()), ..SearchResults::default() }),
    );
    assert_eq!(p.search_status(Engine::Web), (Some(&Text::Raw("Try other keywords".into())), false));
    let effects = p.search(Engine::Web);
    let PickerEffect::Search { generation, .. } = effects[0].clone() else { panic!() };
    p.search_result(Engine::Web, generation, 1, Err(Text::key("dialogs.image_picker.search_failed")));
    assert_eq!(p.search_status(Engine::Web), (Some(&Text::key("dialogs.image_picker.search_failed")), true));
}

// FE-331: Google pages append, load more only when idle with results.
#[test]
fn google_load_more_appends() {
    let mut p = picker();
    p.select_source(Source::Google);
    p.set_query(Engine::Google, "cats");
    let effects = p.search(Engine::Google);
    let PickerEffect::Search { generation, .. } = effects[0].clone() else { panic!() };
    p.search_result(
        Engine::Google,
        generation,
        1,
        Ok(SearchResults { images: vec![web("https://g/1.jpg", None)], total: Some(42), next_start: Some(11), ..SearchResults::default() }),
    );
    assert_eq!(
        p.search_status(Engine::Google).0,
        Some(&Text::with("dialogs.image_picker.search_result_many", &[("count", "42")]))
    );
    let effects = p.load_more(Engine::Google);
    let [PickerEffect::Search { generation, start: 11, .. }] = effects.as_slice() else { panic!("{effects:?}") };
    assert!(p.load_more(Engine::Google).is_empty(), "not while loading");
    p.search_result(Engine::Google, *generation, 11, Ok(results(vec![web("https://g/2.jpg", None)])));
    assert_eq!(p.search_cards(Engine::Google).len(), 2);
    let mut off = ImagePicker::default();
    off.open(PickerConfig { google_enabled: false, ..config() }, None, Provider::Local, false, today());
    assert!(off.config().needs_setup(Source::Google));
    off.set_query(Engine::Google, "x");
    assert!(off.search(Engine::Google).is_empty());
}

// folder_images rows 4, 5, 8, 14; folder_images_controller rows 1-2
#[test]
fn folder_source_choose_cancel_errors_and_filter() {
    let mut p = picker();
    p.select_source(Source::Folder);
    assert_eq!(p.choose_folder(), vec![PickerEffect::ChooseFolder]);
    assert!(p.folder_chosen(Ok(None)).is_empty(), "cancel is silent");
    assert_eq!(p.folder_error(), None);
    p.folder_chosen(Err(Text::Raw("denied".into())));
    assert_eq!(p.folder_error(), Some(&Text::key("dialogs.image_picker.folder_access_failed")));
    let effects = p.folder_chosen(Ok(Some(PathBuf::from("/pics"))));
    let [PickerEffect::ReadFolder { generation, .. }] = effects.as_slice() else { panic!() };
    p.folder_listing(*generation, Err(Text::Raw("io".into())));
    assert_eq!(p.folder_error(), Some(&Text::key("dialogs.image_picker.folder_read_failed")));
    assert!(p.folder_cards().is_empty());
    let effects = p.folder_chosen(Ok(Some(PathBuf::from("/pics"))));
    let PickerEffect::ReadFolder { generation, .. } = effects[0].clone() else { panic!() };
    p.folder_listing(generation, Ok(vec![file("cat.png", 1), file("dog.png", 3), file("notes.txt", 5)]));
    let cards = p.folder_cards();
    assert_eq!(cards.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(), ["dog.png", "cat.png"]);
    p.select_card("folder-0");
    assert_eq!(p.selected_name(), Some("dog.png"));
    p.set_folder_search("cat");
    assert_eq!(p.selected_name(), None, "a new filter clears the selection");
    assert_eq!(p.folder_cards().len(), 1);
    p.set_folder_search("zebra");
    assert!(p.folder_empty());
}

// ai_images rows 1-4; FE-333..336
#[test]
fn ai_generate_flow() {
    let mut p = picker();
    let effects = p.select_source(Source::Ai);
    assert_eq!(effects, vec![PickerEffect::LoadRefImages { search: String::new() }]);
    p.ref_images_result(vec![("photos/ref.jpg".into(), "ref.jpg".into()), ("b.jpg".into(), "<script>xss</script>".into())]);
    p.choose_ref("photos/ref.jpg");
    let cards = p.ref_cards();
    assert!(cards[0].selected && !cards[1].selected);
    assert_eq!(cards[1].label, "<script>xss</script>");
    assert_eq!(p.reference(), Some(("photos/ref.jpg", "ref.jpg")));
    assert!(p.generate().is_empty());
    assert_eq!(p.ai_error(), Some(&Text::key("dialogs.image_picker.ai_prompt_required")));
    p.set_prompt("a mountain at sunset");
    let effects = p.generate();
    let [PickerEffect::Generate { generation, reference, .. }] = effects.as_slice() else { panic!() };
    assert_eq!(reference.as_deref(), Some("photos/ref.jpg"));
    assert!(p.generating());
    let result = AiResult { data: "aGVsbG8=".into(), mime: "image/png".into(), model: "m".into(), revised_prompt: None };
    p.generate_result(*generation, Ok(result), 1_730_000_000_000);
    assert_eq!(p.selected_name(), Some("ai_1730000000000.png"));
    assert_eq!(p.alt(), "a mountain at sunset");
    let effects = p.insert(true);
    assert_eq!(
        effects,
        vec![PickerEffect::Store(StoreRequest::Base64 {
            data: "aGVsbG8=".into(),
            mime: "image/png".into(),
            filename: "ai_1730000000000.png".into(),
            s3_prefix: None,
        })]
    );
}

#[test]
fn ai_cancel_and_not_configured() {
    let mut p = picker();
    p.select_source(Source::Ai);
    p.set_prompt("x");
    let effects = p.generate();
    let PickerEffect::Generate { generation, .. } = effects[0].clone() else { panic!() };
    assert_eq!(p.cancel_generate(), vec![PickerEffect::CancelGenerate]);
    assert!(!p.generating());
    p.generate_result(generation, Ok(AiResult { data: String::new(), mime: String::new(), model: String::new(), revised_prompt: None }), 1);
    assert_eq!(p.selected_name(), None, "a cancelled generation's answer is dropped");
    let mut off = ImagePicker::default();
    off.open(PickerConfig { ai_enabled: false, ..config() }, None, Provider::Local, false, today());
    off.set_prompt("x");
    assert!(off.generate().is_empty());
    assert_eq!(off.ai_error(), Some(&Text::key("dialogs.image_picker.ai_not_configured")));
}

#[test]
fn reopen_resets_everything() {
    let mut p = picker();
    p.ingest_external(vec![file("a.png", 1)]);
    p.set_link("https://x");
    p.open(config(), None, Provider::Local, true, today());
    assert_eq!(p.source(), Source::Upload);
    assert_eq!(p.selected_name(), None);
    assert_eq!(p.link(), "");
    assert!(p.upload_cards().is_empty());
}
