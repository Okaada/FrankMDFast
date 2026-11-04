//! The exact text the media dialogs insert (docs/inventory/frontend.md D1.1,
//! D1.2, D1.4) and where it lands. Adapted rows of
//! docs/testing/ui-test-plan.md (app_library, image_picker, video_dialog).
#![allow(clippy::unwrap_used)]

use frankmd_media::MediaKind;
use frankmd_media::vm::markup::*;

// adapts: app_controller.js relativeMediaPath / frontend.md D1.1 example
#[test]
fn local_upload_becomes_note_relative_with_escaped_alt() {
    let md = final_image_markdown(
        Some("blog/post.md"),
        "images/20260704_101500_ab12cd34_photo.png",
        "photo",
        "photo.png",
        "",
    );
    assert_eq!(md.markdown, "![photo](../images/20260704_101500_ab12cd34_photo.png)");
    assert_eq!(md.url, "images/20260704_101500_ab12cd34_photo.png");
}

// adapts: image_picker_controller.test.js:124 "includes the uploaded image URL and Markdown options"
#[test]
fn root_note_keeps_images_path_and_link_wraps() {
    let md = final_image_markdown(Some("note.md"), "images/photo.png", "photo", "photo.png", "");
    assert_eq!(md.markdown, "![photo](images/photo.png)");
    let linked = final_image_markdown(Some("note.md"), "images/photo.png", " photo ", "photo.png", " https://x.y ");
    assert_eq!(linked.markdown, "[![photo](images/photo.png)](https://x.y)");
    assert_eq!(linked.link, "https://x.y");
}

#[test]
fn alt_falls_back_to_name_then_image() {
    assert_eq!(final_image_markdown(None, "https://a/b.png", "  ", "cat.png", "").markdown, "![cat.png](https://a/b.png)");
    assert_eq!(final_image_markdown(None, "https://a/b.png", "", "", "").markdown, "![Image](https://a/b.png)");
}

// FrankMD quirk D1.7: remote/S3 alt is not escaped; only images/ URLs are rebuilt.
#[test]
fn remote_alt_is_not_escaped_but_local_alt_is() {
    let remote = final_image_markdown(Some("a/n.md"), "https://h/x.png", "a [b]", "x", "");
    assert_eq!(remote.markdown, "![a [b]](https://h/x.png)");
    let local = final_image_markdown(Some("a/n.md"), "images/x y.png", "a [b]\nc", "x", "");
    assert_eq!(local.markdown, "![a \\[b\\] c](../images/x%20y.png)");
}

#[test]
fn relative_media_path_rules() {
    assert_eq!(relative_media_path("note.md", "images/a.png").as_deref(), Some("images/a.png"));
    assert_eq!(relative_media_path("a/b/note.md", "images/a.png").as_deref(), Some("../../images/a.png"));
    assert_eq!(relative_media_path("images/sub/n.md", "images/sub/a.png").as_deref(), Some("a.png"));
    assert_eq!(relative_media_path("images/n.md", "images/sub/a.png").as_deref(), Some("sub/a.png"));
    assert_eq!(relative_media_path("a/../n.md", "images/a.png"), None);
    assert_eq!(relative_media_path("n.md", "other/a.png"), None);
    assert_eq!(relative_media_path("n.md", "images"), None);
    assert_eq!(relative_media_path("n.md", "images//a.png"), None);
}

#[test]
fn encoding_and_escaping_match_the_js() {
    assert_eq!(encode_relative_media_path("../images/photo (one)!*'.png"), "../images/photo%20%28one%29%21%2A%27.png");
    assert_eq!(encode_relative_media_path("images/café.png"), "images/caf%C3%A9.png");
    assert_eq!(escape_markdown_alt("a\\b[c]\r\n\nd"), "a\\\\b\\[c\\] d");
    assert_eq!(safe_remote_media_url("https://cdn.x/a (1).png").as_deref(), Some("https://cdn.x/a%20%281%29.png"));
    assert_eq!(safe_remote_media_url("https://u:p@x/a.png"), None);
    assert_eq!(safe_remote_media_url("javascript:alert(1)"), None);
    assert_eq!(safe_remote_media_url("file:///etc/passwd"), None);
}

#[test]
fn default_alts() {
    assert_eq!(file_alt("my-cat_photo.final.png"), "my cat photo.final");
    assert_eq!(file_alt("noext"), "noext");
    assert_eq!(web_alt("a-b_c"), "a b c");
    assert_eq!(web_alt(&"x".repeat(150)).len(), 100);
    assert_eq!(ai_alt("short"), "short");
    let long = "p".repeat(120);
    assert_eq!(ai_alt(&long), format!("{}...", "p".repeat(100)));
    // UTF-16 code units, like JS substring.
    assert_eq!(js_prefix("😀😀", 3), "😀");
}

// adapts: app_library.test.js row 5 "Insert from a card puts a note-relative link"
#[test]
fn library_image_insert_is_note_relative() {
    let entry = LibraryEntry {
        name: "photo one.png".into(),
        path: "images/photo (one).png".into(),
        kind: MediaKind::Image,
        s3_url: None,
    };
    let insert = library_insert(&entry, Some("folder/sub/note.md")).unwrap();
    assert_eq!(insert, MediaInsert::Image("![photo one.png](../../images/photo%20%28one%29.png)".into()));
}

#[test]
fn library_video_and_s3_inserts() {
    let video = LibraryEntry { name: "clip.mp4".into(), path: "videos/clip.MP4".into(), kind: MediaKind::Video, s3_url: None };
    assert_eq!(
        library_insert(&video, Some("n.md")).unwrap(),
        MediaInsert::Block("<video controls class=\"video-player\">\n  <source src=\"videos/clip.MP4\" type=\"video/mp4\">\n</video>".into())
    );
    let unknown = LibraryEntry { name: "x.flv".into(), path: "videos/x.flv".into(), kind: MediaKind::Video, s3_url: None };
    assert_eq!(
        library_insert(&unknown, Some("n.md")).unwrap(),
        MediaInsert::Block("<video controls class=\"video-player\">\n  <source src=\"videos/x.flv\">\n</video>".into())
    );
    let s3 = LibraryEntry {
        name: "a [1].png".into(),
        path: "frankmd/2026/07/a (1).png".into(),
        kind: MediaKind::Image,
        s3_url: Some("https://b.s3.amazonaws.com/frankmd/2026/07/a (1).png".into()),
    };
    assert_eq!(
        library_insert(&s3, Some("x/n.md")).unwrap(),
        MediaInsert::Image("![a \\[1\\].png](https://b.s3.amazonaws.com/frankmd/2026/07/a%20%281%29.png)".into())
    );
}

// adapts: app_library.test.js row 6 "Insert explains why it can't run"
#[test]
fn library_insert_refusals() {
    let entry = LibraryEntry { name: "a.png".into(), path: "images/a.png".into(), kind: MediaKind::Image, s3_url: None };
    assert_eq!(library_insert(&entry, None), Err(LibraryInsertError::NoOpenNote));
    assert_eq!(library_insert(&entry, Some(".fed")), Err(LibraryInsertError::MarkdownNoteRequired));
    assert_eq!(LibraryInsertError::NoOpenNote.key(), "library.no_open_note");
    let bad = LibraryEntry { path: "videos/a.png".into(), ..entry.clone() };
    assert_eq!(library_insert(&bad, Some("n.md")), Err(LibraryInsertError::InvalidMedia));
    let traversal = LibraryEntry { path: "images/../a.png".into(), ..entry.clone() };
    assert!(!is_valid_library_entry(&traversal));
    let s3_creds = LibraryEntry { path: "frankmd/a.png".into(), s3_url: Some("https://u:p@h/a.png".into()), ..entry };
    assert!(!is_valid_library_entry(&s3_creds));
}

// adapts: video_dialog_controller.test.js rows 22-23 (embed formats, FE-343)
#[test]
fn youtube_embeds_are_byte_exact() {
    assert_eq!(
        youtube_embed("xyz789", None, false),
        "<div class=\"embed-container\">\n  <iframe\n    src=\"https://www.youtube.com/embed/xyz789\"\n    title=\"YouTube video player\"\n    frameborder=\"0\"\n    allow=\"accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share\"\n    referrerpolicy=\"strict-origin-when-cross-origin\"\n    allowfullscreen>\n  </iframe>\n</div>"
    );
    assert!(youtube_embed("xyz789", Some("Test \"Title\""), false).contains("title=\"Test \"Title\"\""));
    assert_eq!(youtube_embed("xyz789", Some("Test Title"), true), "{{< youtube id=\"xyz789\" title=\"Test Title\" >}}");
    assert_eq!(youtube_embed("xyz789", None, true), "{{< youtube id=\"xyz789\" >}}");
    assert_eq!(youtube_embed("xyz789", Some(YOUTUBE_DEFAULT_TITLE), true), "{{< youtube id=\"xyz789\" >}}");
}

// FE-344: file embed with escaped URL and MIME by extension.
#[test]
fn video_file_embed_is_byte_exact() {
    assert_eq!(
        video_file_embed("/v/a\"b.webm"),
        "<video controls class=\"video-player\">\n  <source src=\"/v/a&quot;b.webm\" type=\"video/webm\">\n  Your browser does not support the video tag.\n</video>"
    );
    assert!(video_file_embed("clip.MKV").contains("type=\"video/x-matroska\""));
    assert!(video_file_embed("clip.xyz").contains("type=\"video/mp4\""));
    assert_eq!(video_mime("ogv"), Some("video/ogg"));
}

// FE-313: image insertion adds single newlines only when needed.
#[test]
fn image_insertion_pads_with_single_newlines() {
    let image = MediaInsert::Image("![a](b)".into());
    let edit = insertion_edit("abcdef", 3..3, None, &image).unwrap();
    assert_eq!((edit.range.clone(), edit.text.as_str()), (3..3, "\n![a](b)\n"));
    assert_eq!(edit.caret, 3 + "\n![a](b)\n".len());
    let edit = insertion_edit("ab\n\ncd", 4..4, None, &image).unwrap();
    assert_eq!(edit.text, "![a](b)\n");
    let edit = insertion_edit("", 0..0, None, &image).unwrap();
    assert_eq!(edit.text, "![a](b)");
    // Replaces the selection.
    let edit = insertion_edit("x SEL y", 2..5, None, &image).unwrap();
    assert_eq!((edit.range, edit.text.as_str()), (2..5, "\n![a](b)\n"));
}

// FE-345 + slash commands: block padding and the /query range absorbing blanks.
#[test]
fn block_insertion_normalizes_blank_lines_and_slash_range() {
    let block = MediaInsert::Block("<video>".into());
    assert_eq!(insertion_edit("", 0..0, None, &block).unwrap().text, "<video>");
    assert_eq!(insertion_edit("para", 4..4, None, &block).unwrap().text, "\n\n<video>");
    assert_eq!(insertion_edit("para\n", 5..5, None, &block).unwrap().text, "\n<video>");
    assert_eq!(insertion_edit("a\n\nb", 3..3, None, &block).unwrap().text, "<video>\n\n");
    let text = "intro\n  /video  \nend";
    let slash = 8..14;
    let edit = insertion_edit(text, 0..0, Some(slash), &block).unwrap();
    let mut out = text.to_owned();
    out.replace_range(edit.range.clone(), &edit.text);
    assert_eq!(out, "intro\n\n<video>\n\nend");
}

#[test]
fn labels() {
    assert_eq!(format_bytes(0), "0 B");
    assert_eq!(format_bytes(1023), "1023 B");
    assert_eq!(format_bytes(2048), "2 KB");
    assert_eq!(format_bytes(1536), "1.5 KB");
    assert_eq!(format_bytes(15 * 1024 + 300), "15 KB");
    assert_eq!(format_bytes(5 * 1024 * 1024), "5 MB");
    assert_eq!(dims_badge(Some((800, 600))).as_deref(), Some("800x600"));
    assert_eq!(dims_badge(Some((0, 600))), None);
    assert_eq!(aspect_ratio(1920, 1080).as_deref(), Some("16:9"));
    assert_eq!(aspect_ratio(0, 1), None);
    assert_eq!(format_label("photo.png").as_deref(), Some("PNG"));
    assert_eq!(format_label("noext"), None);
}
