use include_str::include_str;

#[test]
fn suffix_removal_works_in_string_and_line_pipelines() {
    const SUFFIX: &str = "!";
    const TEXT: &str = include_str::include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/message.txt") => strip_line_suffix(SUFFIX));
    static STATIC_TEXT: &str = include_str::include_str!(
        "fixtures/message.txt" => strip_line_suffix(SUFFIX,),
    );
    assert_eq!(TEXT, " \tHello, world\r\n");
    assert_eq!(STATIC_TEXT, TEXT);
    assert_eq!(
        include_str::include_lines!("fixtures/message.txt" => trim_lines => strip_line_suffix("!")),
        &["Hello, world"]
    );
    assert_eq!(
        include_str::include_str!("fixtures/message.txt" => strip_line_suffix("!") => replace("!", "?")),
        " \tHello, world\r\n"
    );
    assert_eq!(
        include_str::include_str!("fixtures/message.txt" => replace("!", "?") => strip_line_suffix("!")),
        " \tHello, world?\r\n"
    );
    assert_eq!(
        include_str::include_str!("fixtures/empty.txt" => strip_line_suffix("")),
        ""
    );
}

#[test]
fn jsonc_converts_files_in_const_and_static_contexts() {
    const JSON: &str = include_str::include_str!("fixtures/config.jsonc" => jsonc);
    static ABSOLUTE: &str = include_str::include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/config.jsonc"
    ) => jsonc);
    assert_eq!(
        JSON,
        r#"{"url":"https://example.test/a/*b*/","values":[1,true,null]}"#
    );
    assert_eq!(ABSOLUTE, JSON);
    assert_eq!(
        include_str::include_str!("fixtures/config.json" => jsonc),
        include_str!("fixtures/config.json" => json)
    );
}

#[test]
fn replacements_work_in_constants_and_statics() {
    const FROM: &str = "world";
    const TO: &str = "🦀 Rust";
    const TEXT: &str = include_str!("fixtures/message.txt" => replace(FROM, TO));
    static EMPTY: &str = include_str!("fixtures/empty.txt" => replace("", "é"));
    assert_eq!(TEXT, " \tHello, 🦀 Rust!\r\n");
    assert_eq!(EMPTY, "é");
    assert_eq!(
        include_str!("fixtures/unicode.txt" => replace("", "-")),
        include_str::include_str!("fixtures/unicode.txt").replace("", "-")
    );
    assert_eq!(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/message.txt") => replace("Hello", concat!("Hi", "!"))),
        " \tHi!, world!\r\n"
    );
}

#[test]
fn multiple_replacements_run_in_order_at_compile_time() {
    const FROM: &str = "world";
    const TEXT: &str = include_str!("fixtures/message.txt" => replace(FROM, concat!("Rust", "🦀")) => replace("Hello", "Hi") => replace("Rust🦀", "friends"));
    assert_eq!(TEXT, " \tHi, friends!\r\n");
    static EMPTY: &str =
        include_str!("fixtures/empty.txt" => replace("", "é") => replace("é", "🦀"));
    assert_eq!(EMPTY, "🦀");
    assert_eq!(
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/unicode.txt") => replace("", "-") => replace("-", "é") => replace("é", "")),
        include_str::include_str!("fixtures/unicode.txt")
            .replace("", "-")
            .replace("-", "é")
            .replace("é", "")
    );
    assert_eq!(
        include_str!("fixtures/message.txt" => replace("world", "worldworld") => replace("absent", "x")),
        " \tHello, worldworld!\r\n"
    );
}

#[test]
fn prefix_removal_and_json_work_through_public_macros() {
    const PREFIX: &str = "  ";
    const TEXT: &str = include_str!("fixtures/unicode.txt" => strip_line_prefix(PREFIX));
    assert_eq!(TEXT, "\u{2003}\tcafé 🦀\n日本語\u{a0}");
    assert_eq!(
        include_str!("fixtures/empty.txt" => strip_line_prefix("")),
        ""
    );
    static JSON: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/config.json"
    ) => json);
    assert_eq!(
        JSON,
        r#"{"message":"two  spaces","enabled":true,"values":[1,null]}"#
    );
}
