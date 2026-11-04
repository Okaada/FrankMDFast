//! Content-loss guard (`autosave_controller.js#isLargeDeletion`) — the
//! threshold logic behind autosave_controller.test.js's saveNow() tests.
//! Rule: `baseline.length > 0 && lost/baseline.length > 0.2 && lost > 50`,
//! lengths in UTF-16 code units, both comparisons strict.

use frankmd_core::guard::{DeletionVerdict, check_deletion, is_large_deletion};

fn x(n: usize) -> String {
    "x".repeat(n)
}

// port: test/javascript/controllers/autosave_controller.test.js:142 "no warning for small deletion (< 20% or < 50 chars)"
#[test]
fn small_deletion_is_ok() {
    // 10 % loss, 20 chars lost
    assert_eq!(check_deletion(Some(&x(200)), &x(180)), DeletionVerdict::Ok);
}

// port: test/javascript/controllers/autosave_controller.test.js:155 "shows warning for large deletion (> 20% AND > 50 chars)"
#[test]
fn large_deletion_warns_with_removed_count() {
    assert_eq!(
        check_deletion(Some(&x(300)), ""),
        DeletionVerdict::LargeDeletion { removed_chars: 300 }
    );
}

// port: test/javascript/controllers/autosave_controller.test.js:185 "no warning when _lastSavedContent is null (fresh file)"
#[test]
fn no_baseline_never_warns() {
    assert!(!is_large_deletion(None, ""));
    assert!(
        !is_large_deletion(Some(""), ""),
        "empty baseline short-circuits"
    );
}

// contract: both thresholds are required and strict
#[test]
fn thresholds_are_strict_and_both_required() {
    // > 20 % but only 50 lost (not > 50)
    assert!(!is_large_deletion(Some(&x(100)), &x(50)));
    // 51 lost and 51 % → warns
    assert!(is_large_deletion(Some(&x(100)), &x(49)));
    // exactly 20 % (60 of 300) → no warning; 61 of 300 → warns
    assert!(!is_large_deletion(Some(&x(300)), &x(240)));
    assert!(is_large_deletion(Some(&x(300)), &x(239)));
    // many chars lost but under 20 % → fine
    assert!(!is_large_deletion(Some(&x(10_000)), &x(9_000)));
    // growth / replacement of equal length is never a deletion
    assert!(!is_large_deletion(Some(&x(300)), &x(400)));
    assert!(!is_large_deletion(Some(&x(300)), &"y".repeat(300)));
}

// contract: lengths are JS `.length` (UTF-16 code units), not bytes or scalars
#[test]
fn lengths_are_utf16_code_units() {
    // 40 emoji = 80 UTF-16 units (160 UTF-8 bytes). Deleting all of them loses 80 > 50.
    let emoji = "😀".repeat(40);
    assert_eq!(
        check_deletion(Some(&emoji), ""),
        DeletionVerdict::LargeDeletion { removed_chars: 80 }
    );
    // 30 'é' = 30 units (60 bytes): losing all is only 30 → ok (a byte count would say 60).
    assert!(!is_large_deletion(Some(&"é".repeat(30)), ""));
}
