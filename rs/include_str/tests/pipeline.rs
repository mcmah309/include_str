use include_str::include_str;

#[test]
fn operations_compose_left_to_right() {
    const TEXT: &str = include_str!(
        "fixtures/message.txt" =>
        collapse_whitespace(space) =>
        trim =>
        replace("world", "Rust"),
    );
    assert_eq!(TEXT, "Hello, Rust!");
    assert_eq!(
        include_str!("fixtures/message.txt" => trim => replace("Hello", " Hello")),
        " Hello, world!"
    );
    assert_eq!(
        include_str!("fixtures/message.txt" => replace("Hello", " Hello") => trim),
        "Hello, world!"
    );
    assert_eq!(
        include_str!(
            "fixtures/message.txt" =>
            strip_line_prefix(" \t",) =>
            trim_lines =>
            collapse_whitespace =>
            replace("world", "Rust",) =>
            replace("Rust", "🦀") =>
            trim,
        ),
        "Hello, 🦀!"
    );
}

#[test]
fn individual_operations_match_named_macros() {
    macro_rules! same {
        ($path:literal, $named:ident, $($op:tt)+) => {
            assert_eq!(include_str!($path => $($op)+), include_str::$named!($path));
        };
    }
    same!("fixtures/message.txt", include_str_trim, trim);
    same!("fixtures/message.txt", include_str_trim_lines, trim_lines);
    same!(
        "fixtures/message.txt",
        include_str_collapse_whitespace,
        collapse_whitespace
    );
    same!(
        "fixtures/message.txt",
        include_str_collapse_whitespace_as_space,
        collapse_whitespace(space)
    );
    same!("fixtures/query.sql", include_sql_str, sql);
    same!("fixtures/config.json", include_str_json, json);
    same!("fixtures/config.jsonc", include_str_jsonc, jsonc);
    assert_eq!(
        include_str!("fixtures/message.txt",),
        core::include_str!("fixtures/message.txt")
    );
    assert_eq!(
        include_str!("fixtures/message.txt" => replace("world", "Rust")),
        include_str::include_str_replace!("fixtures/message.txt", "world", "Rust")
    );
    assert_eq!(
        include_str!("fixtures/message.txt" => strip_line_prefix(" \t")),
        include_str::include_str_strip_line_prefix!("fixtures/message.txt", " \t")
    );
}

#[test]
fn supports_constant_arguments_paths_and_empty_results() {
    const FROM: &str = "world";
    const TO: &str = "Rust";
    static TEXT: &str = include_str!(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/message.txt") =>
        trim =>
        replace(FROM, TO),
    );
    assert_eq!(TEXT, "Hello, Rust!");
    assert_eq!(
        include_str!("fixtures/empty.txt" => trim => collapse_whitespace(space)),
        ""
    );
    assert_eq!(
        include_str!("fixtures/whitespace.txt" => collapse_whitespace => trim),
        ""
    );
    assert_eq!(
        include_str!("fixtures/empty.txt" => replace("", " x ") => trim),
        "x"
    );
    assert_eq!(
        include_str!("fixtures/config.jsonc" => jsonc => json),
        include_str::include_str_jsonc!("fixtures/config.jsonc")
    );
}
