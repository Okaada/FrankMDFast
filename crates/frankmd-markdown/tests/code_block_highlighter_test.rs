//! Port of `test/javascript/lib/code_block_highlighter.test.js` (28 tests;
//! 27 ported, 1 skipped).
//!
//! Lezer `tok-*` classes are asserted as the theme roles FrankMD's `prose.css`
//! maps them to ([`TokenClass`] + the `tok-definition` flag); see
//! `ir::TokenClass` for the table. Budget tests run the real render path.
//!
//! Skipped: :365 "does not apply a lazy parser result to code replaced by a
//! newer render" — a DOM race between async parser imports and innerHTML
//! replacement. Natively a render is one synchronous pure call whose result
//! replaces the previous `RenderDoc` as a whole (stale results are dropped by
//! generation in the app, HP-017), so there is nothing to apply late.
#![allow(clippy::unwrap_used)]

mod common;

use std::sync::Arc;

use common::*;
use frankmd_markdown::highlight::{BuiltinHighlighter, language_for_fence};
use frankmd_markdown::html::to_html;
use frankmd_markdown::ir::TokenClass::{
    Comment, Constant, Function, Identifier, Keyword, Number, Operator, Property, String as Str,
    Type,
};
use frankmd_markdown::{RenderDoc, RenderOptions, render_body};

fn label(alias: &str) -> Option<&'static str> {
    language_for_fence(alias).map(|l| l.label)
}

fn render_hl(md: &str) -> RenderDoc {
    render_body(
        md,
        0,
        &RenderOptions {
            highlighter: Some(Arc::new(BuiltinHighlighter)),
            ..RenderOptions::default()
        },
    )
}

fn fence(lang: &str, code: &str) -> String {
    format!("```{lang}\n{code}```\n")
}

// port: test/javascript/lib/code_block_highlighter.test.js:11 "highlights JavaScript and TypeScript tokens"
#[test]
fn javascript_and_typescript_tokens() {
    let t = tokens(
        "typescript",
        "const x: number = 1; describe(\"suite\", () => { expect(x).toBe(true); });\n",
    );
    assert_token(&t, "const", Keyword, false);
    assert_token(&t, "1", Number, false);
    assert_token(&t, "describe", Function, false);
    assert_token(&t, "expect", Function, false);
    assert_token(&t, "toBe", Function, false);
    assert_token(&t, "true", Constant, false);
    assert_eq!(label("typescript"), Some("TypeScript"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:26 "highlights comments and object properties with distinct token roles"
#[test]
fn comments_and_object_properties() {
    let t = tokens("js", "// explanation\nconst item = { name: \"value\" };\n");
    assert_token(&t, "// explanation", Comment, false);
    assert_token(&t, "name", Property, true);
}

// port: test/javascript/lib/code_block_highlighter.test.js:35 "highlights YAML keys, comments, scalar values, and yml fences"
#[test]
fn yaml_tokens() {
    let t = tokens(
        "yaml",
        "# app settings\napp:\n  port: 3000\n  enabled: true\n  message: \"ready\"\n",
    );
    assert_token(&t, "# app settings", Comment, false);
    assert_token(&t, "app", Property, true);
    assert_token(&t, "3000", Number, false);
    assert_token(&t, "true", Constant, false);
    assert_token(&t, "\"ready\"", Str, false);
    assert_eq!(label("yaml"), Some("YAML"));
    assert_eq!(label("yml"), Some("YAML"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:48 "highlights Rust keywords, types, functions, strings, and numbers"
#[test]
fn rust_tokens() {
    let t = tokens(
        "rust",
        "// entry point\nfn main() { let answer: i32 = 42; println!(\"hello\"); }",
    );
    assert_token(&t, "// entry point", Comment, false);
    assert_token(&t, "fn", Keyword, false);
    assert_token(&t, "main", Function, true);
    assert_token(&t, "i32", Type, false);
    assert_token(&t, "42", Number, false);
    assert_token(&t, "\"hello\"", Str, false);
    assert_eq!(label("rust"), Some("Rust"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:62 "highlights Go keywords, functions, strings, and numbers"
#[test]
fn go_tokens() {
    let t = tokens(
        "go",
        "package main\n\nimport \"fmt\"\n\nfunc Greet(name string) string { return fmt.Sprintf(\"hello, %s\", name) }\nconst answer = 42",
    );
    assert_token(&t, "package", Keyword, false);
    assert_token(&t, "func", Keyword, false);
    assert_token(&t, "Greet", Function, true);
    assert_token(&t, "Sprintf", Property, false);
    assert_token(&t, "\"hello, %s\"", Str, false);
    assert_token(&t, "42", Number, false);
    assert_eq!(label("go"), Some("Go"));
    assert_eq!(label("golang"), Some("Go"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:77 "highlights Python comments, keywords, functions, types, strings, and numbers"
#[test]
fn python_tokens() {
    let t = tokens(
        "python",
        "# greeting\ndef greet(name: str) -> str:\n    return f\"hello {name}\"\n\nanswer = 42",
    );
    assert_token(&t, "# greeting", Comment, false);
    assert_token(&t, "def", Keyword, false);
    assert_token(&t, "greet", Function, true);
    assert_token(&t, "str", Identifier, false);
    assert!(t.iter().any(|(_, s)| s.class == Str), "{t:?}");
    assert_token(&t, "42", Number, false);
    assert_eq!(label("python"), Some("Python"));
    assert_eq!(label("py"), Some("Python"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:92 "highlights Java comments, keywords, definitions, types, strings, and numbers"
#[test]
fn java_tokens() {
    let t = tokens(
        "java",
        "// greeting\npublic class Greeter {\n  public static String greet(String name) {\n    int answer = 42;\n    return \"Hello, \" + name;\n  }\n}",
    );
    assert_token(&t, "// greeting", Comment, false);
    assert_token(&t, "class", Keyword, false);
    assert_token(&t, "Greeter", Identifier, true);
    assert_token(&t, "greet", Identifier, true);
    assert_token(&t, "\"Hello, \"", Str, false);
    assert_token(&t, "42", Number, false);
    assert_eq!(label("java"), Some("Java"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:106 "highlights PHP comments, function definitions, types, strings, and calls"
#[test]
fn php_tokens() {
    let t = tokens(
        "php",
        "<?php\n// greeting\nfunction greet(string $name): string {\n  return \"Hello, \" . $name;\n}\necho greet(\"world\");",
    );
    assert_token(&t, "// greeting", Comment, false);
    assert_token(&t, "function", Keyword, false);
    assert_token(&t, "greet", Function, true);
    assert_token(&t, "string", Type, false);
    assert_token(&t, "$name", Identifier, false);
    assert_token(&t, "\"Hello, \"", Str, false);
    assert_token(&t, "greet", Function, false);
    assert_eq!(label("php"), Some("PHP"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:120 "highlights Kotlin comments, declarations, types, strings, and aliases"
#[test]
fn kotlin_tokens() {
    let t = tokens(
        "kotlin",
        "// greeting\ndata class Greeter(val name: String) {\n  fun greet() = \"Hello, $name\"\n}",
    );
    assert_token(&t, "// greeting", Comment, false);
    assert_token(&t, "data", Keyword, false);
    assert_token(&t, "class", Keyword, false);
    assert_token(&t, "Greeter", Identifier, true);
    assert_token(&t, "String", Type, false);
    assert_token(&t, "\"Hello, $name\"", Str, false);
    assert_eq!(label("kotlin"), Some("Kotlin"));
    assert_eq!(label("kt"), Some("Kotlin"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:134 "highlights Ruby comments, class names, method definitions, strings, and numbers"
#[test]
fn ruby_tokens() {
    let t = tokens(
        "ruby",
        "# greeting\nclass Greeter\n  def greet(name)\n    puts \"hello #{name}\"\n    42\n  end\nend",
    );
    assert_token(&t, "# greeting", Comment, false);
    assert_token(&t, "class", Keyword, false);
    assert_token(&t, "Greeter", Type, false);
    assert_token(&t, "def", Keyword, false);
    assert_token(&t, "greet", Identifier, true);
    assert_token(&t, "\"hello #{", Str, false);
    assert_token(&t, "42", Number, false);
    assert_eq!(label("ruby"), Some("Ruby"));
    assert_eq!(label("rb"), Some("Ruby"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:149 "highlights dotenv comments, variable assignments, strings, and numeric values"
#[test]
fn dotenv_tokens() {
    let t = tokens(
        "dotenv",
        "# local settings\nDATABASE_URL=\"postgres://db/app\"\nPORT=5432\nDEBUG=true\n",
    );
    assert_token(&t, "# local settings", Comment, false);
    assert_token(&t, "DATABASE_URL", Identifier, true);
    assert_token(&t, "\"postgres://db/app\"", Str, false);
    assert_token(&t, "5432", Number, false);
    assert_eq!(label("dotenv"), Some("Dotenv"));
    assert_eq!(label("env"), Some("Dotenv"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:161 "highlights common shell fences and aliases"
#[test]
fn shell_tokens() {
    let t = tokens(
        "bash",
        "# clean temporary files\nfor file in \"$@\"; do\n  echo \"$file\"\ndone",
    );
    assert_token(&t, "# clean temporary files", Comment, false);
    assert_token(&t, "for", Keyword, false);
    assert_token(&t, "\"", Str, false);
    assert_token(&t, "$@", Identifier, true);
    assert_token(&t, "echo", Identifier, false);
    assert_eq!(label("bash"), Some("Shell"));
    assert_eq!(label("sh"), Some("Shell"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:174 "highlights SQL comments, keywords, literals, operators, and dialect aliases"
#[test]
fn sql_tokens() {
    let t = tokens(
        "sql",
        "-- active users\nSELECT id, name FROM users WHERE active = TRUE AND score >= 42;",
    );
    assert_token(&t, "-- active users", Comment, false);
    assert_token(&t, "SELECT", Keyword, false);
    assert_token(&t, "FROM", Keyword, false);
    assert_token(&t, "TRUE", Constant, false);
    assert_token(&t, ">=", Operator, false);
    assert_token(&t, "42", Number, false);
    assert_eq!(label("sql"), Some("SQL"));
    assert_eq!(label("postgresql"), Some("PostgreSQL"));
    assert_eq!(label("mariadb"), Some("MariaDB"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:189 "highlights TOML comments, sections, keys, strings, numbers, and aliases"
#[test]
fn toml_tokens() {
    let t = tokens(
        "toml",
        "# server settings\n[server]\nhost = \"localhost\"\nport = 5432\nenabled = true",
    );
    assert_token(&t, "# server settings", Comment, false);
    assert_token(&t, "[server]", Constant, false);
    assert_token(&t, "host", Property, false);
    assert_token(&t, "\"localhost\"", Str, false);
    assert_token(&t, "5432", Number, false);
    assert_token(&t, "true", Constant, false);
    assert_eq!(label("toml"), Some("TOML"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:202 "highlights Dockerfile comments and build instructions"
// (the shipped app never loaded this parser — see js_code_block_highlighter.json
// notes; the test suite's intent, highlighting Dockerfile, is what we keep)
#[test]
fn dockerfile_tokens() {
    let t = tokens(
        "dockerfile",
        "# minimal image\nFROM node:22-alpine\nWORKDIR /app\nCOPY package*.json ./\nRUN npm install",
    );
    assert_token(&t, "# minimal image", Comment, false);
    assert_token(&t, "FROM", Keyword, false);
    assert_token(&t, "WORKDIR", Keyword, false);
    assert_token(&t, "RUN", Keyword, false);
    assert_eq!(label("dockerfile"), Some("Dockerfile"));
    assert_eq!(label("docker"), Some("Dockerfile"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:214 "highlights Terraform comments, block keywords, attributes, references, and expressions"
#[test]
fn terraform_tokens() {
    let t = tokens(
        "tf",
        "# infrastructure\nresource \"aws_instance\" \"web\" {\n  ami = \"ami-123\"\n  instance_type = var.instance_type\n  count = 2\n  enabled = true\n  size = max(1, 3)\n}",
    );
    assert_token(&t, "# infrastructure", Comment, false);
    assert_token(&t, "resource", Keyword, false);
    assert_token(&t, "ami", Property, false);
    assert_token(&t, "\"ami-123\"", Str, false);
    assert_token(&t, "instance_type", Property, false);
    assert_token(&t, "2", Number, false);
    assert_token(&t, "true", Constant, false);
    assert_token(&t, "max", Function, false);
    assert_eq!(label("tf"), Some("Terraform"));
    assert_eq!(label("hcl"), Some("HCL"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:230 "highlights C and C++ tokens and common fence aliases"
#[test]
fn c_and_cpp_tokens() {
    let t = tokens(
        "cpp",
        "// entry point\nint main() { const int answer = 42; return answer; }",
    );
    assert_token(&t, "// entry point", Comment, false);
    assert_token(&t, "main", Function, true);
    assert_token(&t, "42", Number, false);
    assert_token(&t, "return", Keyword, false);
    assert_eq!(label("cpp"), Some("C++"));
    let c = tokens("c", "int main() { return 0; }");
    assert_token(&c, "0", Number, false);
    assert_eq!(label("c"), Some("C"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:245 "highlights C# keywords, types, definitions, strings, and numbers"
#[test]
fn csharp_tokens() {
    let t = tokens(
        "csharp",
        "// greeting\nusing System;\npublic class Greeter {\n  public static string Greet(string name) {\n    const int answer = 42;\n    return \"Hello, \" + name;\n  }\n}",
    );
    assert_token(&t, "// greeting", Comment, false);
    assert_token(&t, "class", Keyword, false);
    assert_token(&t, "Greeter", Identifier, true);
    assert_token(&t, "\"Hello, \"", Str, false);
    assert_token(&t, "42", Number, false);
    assert_eq!(label("csharp"), Some("C#"));
    assert_eq!(label("cs"), Some("C#"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:258 "supports common aliases and the bundled HTML and CSS parsers"
#[test]
fn common_aliases_html_css() {
    let cases = [
        ("js", "const answer = 42;", Keyword),
        ("ts", "const answer: number = 42;", Type),
        ("jsx", "<Widget title=\"ok\" />", Type),
        ("tsx", "<Widget count={42} />", Type),
        ("json", "{\"answer\": 42}", Str),
        ("jsonc", "{\"answer\": 42}", Str),
        ("html", "<section>text</section>", Type),
        ("css", "a { color: red; }", Type),
    ];
    for (alias, src, class) in cases {
        let t = tokens(alias, src);
        assert!(
            t.iter().any(|(_, s)| s.class == class),
            "{alias}: no {class:?} token in {t:?}"
        );
    }
}

// port: test/javascript/lib/code_block_highlighter.test.js:276 "accepts file-extension-style fence names for config languages"
#[test]
fn extension_style_fence_names() {
    for (fence_name, src, want) in [
        (".env", "PORT=3000", "Dotenv"),
        (".yaml", "answer: 42", "YAML"),
        (".yml", "answer: 42", "YAML"),
        (".rb", "puts 42", "Ruby"),
        (".tf", "count = 1", "Terraform"),
    ] {
        assert_eq!(label(fence_name), Some(want), "{fence_name}");
        let doc = render_hl(&fence(fence_name, &format!("{src}\n")));
        let code = collect_code(&doc);
        assert_eq!(code[0].0.as_deref(), Some(fence_name));
    }
}

// port: test/javascript/lib/code_block_highlighter.test.js:291 "maps HTML attributes and CSS values to their syntax roles"
#[test]
fn html_attributes_and_css_values() {
    let h = tokens("html", "<button title=\"ok\">text</button>");
    assert_token(&h, "title", Property, false);
    assert_token(&h, "\"ok\"", Str, false);
    let c = tokens("css", "a { color: red; margin: 1px; }");
    assert_token(&c, "color", Property, false);
    assert_token(&c, "red", Constant, false);
    assert_token(&c, "1px", Number, false);
}

// port: test/javascript/lib/code_block_highlighter.test.js:305 "leaves unsupported languages unchanged"
#[test]
fn unsupported_language_unchanged() {
    assert_eq!(language_for_fence("unrecognized"), None);
    let doc = render_hl(&fence("unrecognized", "puts \"hello\"\n"));
    let code = collect_code(&doc);
    assert!(!is_highlighted(&code[0].1));
    check_same_structure(
        &to_html(&doc),
        "<pre><code class=\"language-unrecognized\">puts \"hello\"\n</code></pre>",
    )
    .unwrap();
}

// port: test/javascript/lib/code_block_highlighter.test.js:311 "keeps large supported fences readable without highlighting them"
#[test]
fn large_fence_not_highlighted() {
    let source = "const answer = 42;\n".repeat(1100);
    let doc = render_hl(&fence("javascript", &source));
    let code = collect_code(&doc);
    assert!(!is_highlighted(&code[0].1));
    assert_eq!(block_text(&doc.blocks[0]), source);
    assert_eq!(label("javascript"), Some("JavaScript"));
}

// port: test/javascript/lib/code_block_highlighter.test.js:325 "bounds total highlighted source across a preview render"
#[test]
fn total_highlight_budget_per_render() {
    let source = format!("const value = 1;\n{}", "// filler\n".repeat(850));
    let md: String = (0..6)
        .map(|_| fence("js", &source))
        .collect::<Vec<_>>()
        .join("\n");
    let doc = render_hl(&md);
    let code = collect_code(&doc);
    assert_eq!(code.len(), 6);
    assert!(is_highlighted(&code[0].1));
    assert!(is_highlighted(&code[4].1));
    assert!(!is_highlighted(&code[5].1));
    assert_eq!(block_text(&doc.blocks[5]), source);
}

// port: test/javascript/lib/code_block_highlighter.test.js:340 "limits the number of code blocks highlighted per render"
#[test]
fn max_blocks_per_render() {
    let md: String = (0..101)
        .map(|_| fence("js", "const value = 1;\n"))
        .collect::<Vec<_>>()
        .join("\n");
    let doc = render_hl(&md);
    let code = collect_code(&doc);
    assert_eq!(code.len(), 101);
    assert!(is_highlighted(&code[99].1));
    assert!(!is_highlighted(&code[100].1));
}

// port: test/javascript/lib/code_block_highlighter.test.js:351 "highlights common languages before lazy parser imports resolve"
// (category C; adapted: natively every supported language is highlighted in
// the same synchronous render pass — no lazily loaded parsers)
#[test]
fn common_and_lazy_languages_in_one_pass() {
    let doc = render_hl(&format!(
        "{}\n{}",
        fence("js", "const immediate = 1;\n"),
        fence("tf", "value = 1\n")
    ));
    let code = collect_code(&doc);
    assert!(is_highlighted(&code[0].1));
    assert!(is_highlighted(&code[1].1));
}

// port: test/javascript/lib/code_block_highlighter.test.js:377 "keeps code content as text instead of turning it into markup"
#[test]
fn code_content_stays_text() {
    let doc = render_hl(&fence("javascript", "<img src=x onerror=alert(1)>\n"));
    assert!(block_text(&doc.blocks[0]).contains("<img src=x onerror=alert(1)>"));
    let html = to_html(&doc);
    assert!(!html.contains("<img"), "{html}");
    assert!(html.contains("&lt;img"), "{html}");
}
