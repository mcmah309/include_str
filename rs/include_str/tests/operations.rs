use include_str::{include_str_json, include_str_replace, include_str_strip_prefix};

#[test]
fn replacements_work_in_constants_and_statics() {
    const FROM: &str = "world";
    const TO: &str = "🦀 Rust";
    const TEXT: &str = include_str_replace!("fixtures/message.txt", FROM, TO,);
    static EMPTY: &str = include_str_replace!("fixtures/empty.txt", "", "é");
    assert_eq!(TEXT, " \tHello, 🦀 Rust!\r\n");
    assert_eq!(EMPTY, "é");
    assert_eq!(
        include_str_replace!("fixtures/unicode.txt", "", "-"),
        include_str::include_str!("fixtures/unicode.txt").replace("", "-")
    );
    assert_eq!(
        include_str_replace!(
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/message.txt"),
            "Hello",
            concat!("Hi", "!")
        ),
        " \tHi!, world!\r\n"
    );
}

#[test]
fn prefix_removal_and_json_work_through_public_macros() {
    const PREFIX: &str = "  ";
    const TEXT: &str = include_str_strip_prefix!("fixtures/unicode.txt", PREFIX,);
    assert_eq!(TEXT, "\u{2003}\tcafé 🦀\n日本語\u{a0}");
    assert_eq!(include_str_strip_prefix!("fixtures/empty.txt", ""), "");
    static JSON: &str = include_str_json!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/config.json"
    ),);
    assert_eq!(
        JSON,
        r#"{"message":"two  spaces","enabled":true,"values":[1,null]}"#
    );
}
