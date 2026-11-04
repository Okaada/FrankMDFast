//! Port of `test/services/media_usage_service_test.rb` (12 tests).
#![allow(clippy::disallowed_methods)]

#[path = "common/library.rs"]
mod library;

use std::collections::BTreeMap;

use frankmd_media::error::MediaError;
use frankmd_media::usage::MediaUsage;
use library::{Notes, chmod, is_root};

fn index(pairs: &[(&str, &[&str])]) -> BTreeMap<String, Vec<String>> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_owned(), v.iter().map(|s| (*s).to_owned()).collect()))
        .collect()
}

fn build(n: &Notes) -> BTreeMap<String, Vec<String>> {
    (*MediaUsage::new(n.root()).build_index().expect("index")).clone()
}

// port: test/services/media_usage_service_test.rb:15 "indexes Markdown images, image links, and HTML video sources by distinct note"
#[test]
fn indexes_images_links_and_html_video_sources_by_distinct_note() {
    let n = Notes::new();
    n.write(
        "projects/guides/guide.md",
        "![photo](../../images/my%20cat.png)\n\
         [![same photo](../../images/my%20cat.png)](https://example.test/photo)\n\
         ![diagram](../../images/diagram%20%28draft%29.png)\n\
         <video controls><source src=\"../../videos/clips/clip%20one.mp4\" type=\"video/mp4\"></video>\n\
         [download video](../../videos/clips/clip%20one.mp4)\n",
    );
    n.write("index.md", "![photo](images/my%20cat.png)");
    assert_eq!(
        build(&n),
        index(&[
            ("images/diagram (draft).png", &["projects/guides/guide.md"]),
            (
                "images/my cat.png",
                &["index.md", "projects/guides/guide.md"]
            ),
            ("videos/clips/clip one.mp4", &["projects/guides/guide.md"]),
        ])
    );
}

// port: test/services/media_usage_service_test.rb:38 "normalizes Markdown destinations and HTML source URLs"
#[test]
fn normalizes_markdown_destinations_and_html_source_urls() {
    let n = Notes::new();
    n.write(
        "docs/media.md",
        "![cover](../images/cover%20photo.png)\n\
         [open image](/images/cover%20photo.png#full-size)\n\
         <img src=/notes/images/inline%20photo.png>\n\
         [trailer](../videos/trailer.mp4)\n\
         <video src=\"../videos/trailer.mp4\"></video>\n",
    );
    assert_eq!(
        build(&n),
        index(&[
            ("images/cover photo.png", &["docs/media.md"]),
            ("images/inline photo.png", &["docs/media.md"]),
            ("videos/trailer.mp4", &["docs/media.md"]),
        ])
    );
}

// port: test/services/media_usage_service_test.rb:60 "unescapes Markdown punctuation while retaining path containment"
#[test]
fn unescapes_markdown_punctuation_while_retaining_containment() {
    let n = Notes::new();
    n.write(
        "references.md",
        r"![diagram](images/diagram\(draft\).png)
![hash](images/hash\#name.png)
![question](images/question\?name.png)
![reference][diagram]
[diagram]: images/diagram\(draft\).png
![outside](images/\.\./../../outside.png)
",
    );
    assert_eq!(
        build(&n),
        index(&[
            ("images/diagram(draft).png", &["references.md"]),
            ("images/hash#name.png", &["references.md"]),
            ("images/question?name.png", &["references.md"]),
        ])
    );
}

// port: test/services/media_usage_service_test.rb:80 "indexes full, collapsed, and shortcut references with normalized labels"
#[test]
fn indexes_full_collapsed_and_shortcut_references() {
    let n = Notes::new();
    n.write(
        "references.md",
        "![cover][  PHOTO   IMAGE ]\n![video][]\n[shortcut]\n[ photo image ]: images/photo%20one.png\n\
         [video]: videos/clip.mp4\n[shortcut]: images/shortcut.png\n[unused]: images/unused.png\n",
    );
    assert_eq!(
        build(&n),
        index(&[
            ("images/photo one.png", &["references.md"]),
            ("images/shortcut.png", &["references.md"]),
            ("videos/clip.mp4", &["references.md"]),
        ])
    );
}

// port: test/services/media_usage_service_test.rb:104 "does not count an unused reference definition"
#[test]
fn does_not_count_an_unused_reference_definition() {
    let n = Notes::new();
    n.write("unused.md", "[photo]: images/photo.png\n");
    assert!(build(&n).is_empty());
}

// port: test/services/media_usage_service_test.rb:110 "counts duplicate reference uses once per note"
#[test]
fn counts_duplicate_reference_uses_once_per_note() {
    let n = Notes::new();
    n.write(
        "duplicates.md",
        "![first][photo]\n[second][  PHOTO ]\n![photo][]\n[photo]\n[photo]: images/photo.png\n",
    );
    assert_eq!(
        build(&n),
        index(&[("images/photo.png", &["duplicates.md"])])
    );
}

// port: test/services/media_usage_service_test.rb:125 "ignores plain text, unrelated URL substrings, and references inside code"
#[test]
fn ignores_plain_text_unrelated_urls_and_code() {
    let n = Notes::new();
    n.write(
        "examples.md",
        "Plain mention: images/mentioned.png\n\
         [external](https://example.test/notes/images/external.png)\n\
         [near match](notimages/near.png)\n\
         `![inline](images/inline-code.png)`\n\
         \n\
         ```markdown\n\
         ![fenced](images/fenced-code.png)\n\
         <source src=\"videos/fenced-code.mp4\">\n\
         ![fenced reference][fenced]\n\
         [fenced]: images/fenced-reference.png\n\
         ```\n\
         \n\
         <!-- <img src=\"images/commented-out.png\"> -->\n",
    );
    assert!(build(&n).is_empty());
}

// port: test/services/media_usage_service_test.rb:146 "resolves note-relative destinations and rejects paths outside the notes root"
#[test]
fn resolves_note_relative_and_rejects_outside_root() {
    let n = Notes::new();
    n.write(
        "nested/page.md",
        "![photo](../images/photo.png)\n![outside](../../images/outside.png)\n![wrong root](../notes/images/wrong-root.png)\n",
    );
    assert_eq!(
        build(&n),
        index(&[("images/photo.png", &["nested/page.md"])])
    );
}

// port: test/services/media_usage_service_test.rb:159 "ignores hidden notes and symlinked Markdown files"
#[test]
fn ignores_hidden_notes_and_symlinked_markdown() {
    let n = Notes::new();
    n.write(".hidden/hidden.md", "![hidden](images/hidden.png)");
    n.write("visible.md", "![visible](images/visible.png)");
    let outside = n.write_outside("media_usage_outside.md", "![outside](images/outside.png)");
    n.symlink("linked.md", &outside);
    assert_eq!(build(&n), index(&[("images/visible.png", &["visible.md"])]));
}

// port: test/services/media_usage_service_test.rb:172 "raises instead of returning a partial index when a note cannot be read"
#[test]
fn raises_instead_of_partial_index_when_a_note_cannot_be_read() {
    if is_root() {
        eprintln!("skipped: running as root, chmod cannot produce EACCES");
        return;
    }
    let n = Notes::new();
    n.write("readable.md", "![photo](images/photo.png)");
    let unreadable = n.write("unreadable.md", "![other](images/other.png)");
    // `Pathname.any_instance.stubs(:read).raises(Errno::EACCES)` → mode 000.
    let _guard = chmod(&unreadable, 0o000);
    let err = MediaUsage::new(n.root())
        .build_index()
        .expect_err("scan error");
    match err {
        MediaError::Scan(msg) => assert!(msg.contains("Could not read Markdown note"), "{msg}"),
        other => panic!("expected ScanError, got {other:?}"),
    }
}

// port: test/services/media_usage_service_test.rb:182 "reuses an unchanged index without rereading Markdown"
#[test]
fn reuses_an_unchanged_index_without_rereading() {
    let n = Notes::new();
    n.write("stable.md", "![photo](images/photo.png)");
    let usage = MediaUsage::new(n.root());
    let expected = index(&[("images/photo.png", &["stable.md"])]);
    assert_eq!(*usage.build_index().expect("first"), expected);
    assert_eq!(*usage.build_index().expect("second"), expected);
    assert_eq!(usage.build_count(), 1, "build_index_from must run once");
}

// port: test/services/media_usage_service_test.rb:191 "rebuilds the index after a note is edited"
#[test]
fn rebuilds_the_index_after_a_note_is_edited() {
    let n = Notes::new();
    let note = n.write("edited.md", "![photo](images/old.png)");
    let usage = MediaUsage::new(n.root());
    assert_eq!(
        *usage.build_index().expect("first"),
        index(&[("images/old.png", &["edited.md"])])
    );
    std::fs::write(&note, "![photo](images/new.png)").expect("edit");
    assert_eq!(
        *usage.build_index().expect("second"),
        index(&[("images/new.png", &["edited.md"])])
    );
}

// security: a symlinked directory whose target is outside the notes root is never scanned.
#[test]
fn never_scans_symlinked_directories_outside_the_root() {
    let n = Notes::new();
    n.write("visible.md", "![visible](images/visible.png)");
    n.write_outside("escaped/secret.md", "![secret](images/secret.png)");
    n.symlink("linked_dir", &n.outside.path().join("escaped"));
    assert_eq!(build(&n), index(&[("images/visible.png", &["visible.md"])]));
}
