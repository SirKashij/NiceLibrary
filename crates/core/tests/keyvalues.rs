//! The reader for Valve's text KeyValues format (`.vdf`, `.acf`).

use nicelibrary_core::keyvalues::{self, Value};

#[test]
fn reads_nested_blocks() {
    let doc = keyvalues::parse(
        r#"
        "users"
        {
            "76561197960287930"
            {
                "AccountName"   "player_one"
                "MostRecent"    "1"
            }
        }
        "#,
    )
    .unwrap();

    let user = doc.block("users").unwrap().block("76561197960287930").unwrap();
    assert_eq!(user.string("AccountName"), Some("player_one"));
    assert_eq!(user.string("MostRecent"), Some("1"));
}

#[test]
fn looks_up_keys_regardless_of_case() {
    let doc = keyvalues::parse(r#""Users" { "u" { "mostrecent" "1" } }"#).unwrap();

    assert_eq!(doc.block("users").unwrap().block("U").unwrap().string("MostRecent"), Some("1"));
}

#[test]
fn unescapes_paths_and_quotes() {
    let doc = keyvalues::parse(r#""path" "D:\\SteamLibrary"  "name" "say \"hi\"""#).unwrap();

    assert_eq!(doc.string("path"), Some(r"D:\SteamLibrary"));
    assert_eq!(doc.string("name"), Some(r#"say "hi""#));
}

#[test]
fn ignores_comments() {
    let doc = keyvalues::parse(
        "// written by Steam\n\"a\" \"1\" // trailing\n\"b\" { // inside\n \"c\" \"// not a comment\" }\n",
    )
    .unwrap();

    assert_eq!(doc.string("a"), Some("1"));
    assert_eq!(doc.block("b").unwrap().string("c"), Some("// not a comment"));
}

#[test]
fn reads_unquoted_tokens_and_a_byte_order_mark() {
    let doc = keyvalues::parse("\u{feff}AppState { appid 620 }").unwrap();

    assert_eq!(doc.block("appstate").unwrap().string("appid"), Some("620"));
}

#[test]
fn a_comment_ends_an_unquoted_token() {
    let doc = keyvalues::parse("appid 620// the app\nname a/b").unwrap();

    assert_eq!(doc.string("appid"), Some("620"));
    assert_eq!(doc.string("name"), Some("a/b"));
}

#[test]
fn lists_the_entries_of_a_block_in_file_order() {
    let doc = keyvalues::parse(r#""b" "2" "a" { } "c" "3""#).unwrap();

    let keys: Vec<&str> = doc.entries().map(|(key, _)| key).collect();
    assert_eq!(keys, ["b", "a", "c"]);
    assert!(matches!(doc.get("A"), Some(Value::Block(_))));
}

#[test]
fn a_string_is_not_a_block_and_a_block_is_not_a_string() {
    let doc = keyvalues::parse(r#""s" "x" "b" { }"#).unwrap();

    assert!(doc.block("s").is_none());
    assert!(doc.string("b").is_none());
    assert!(doc.get("missing").is_none());
}

#[test]
fn rejects_malformed_input() {
    for (input, why) in [
        (r#""users" {"#, "unclosed block"),
        (r#""users" { "a" "1" } }"#, "stray closing brace"),
        (r#""name" "unterminated"#, "unterminated string"),
        (r#""key""#, "key without value"),
        (r#""users" { "key" }"#, "key without value inside a block"),
        (r#"{ "a" "1" }"#, "block without key"),
    ] {
        assert!(keyvalues::parse(input).is_err(), "accepted {why}: {input}");
    }
}

#[test]
fn says_on_which_line_the_input_is_malformed() {
    let error = keyvalues::parse("\"a\" \"1\"\n\"b\" {\n\"c\" \"2\"\n").unwrap_err();

    assert_eq!(error.line(), 4);
}
