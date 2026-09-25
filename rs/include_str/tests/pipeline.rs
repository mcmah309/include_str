use include_str::include_str;

#[test]
fn custom_whitespace_replacements_are_constant_strings() {
    const SEPARATOR: &str = "🦀 / ";
    static TEXT: &str = include_str!(
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/message.txt")
            => replace_whitespace(SEPARATOR,),
    );
    assert_eq!(TEXT, "🦀 / Hello,🦀 / world!🦀 / ");
    assert_eq!(
        include_str!("fixtures/message.txt" => replace_whitespace(" ")),
        " Hello, world! "
    );
    assert_eq!(
        include_str!("fixtures/message.txt" => replace_whitespace("")),
        "Hello,world!"
    );
    assert_eq!(
        include_str!("fixtures/empty.txt" => replace_whitespace("x")),
        ""
    );
    assert_eq!(
        include_str!("fixtures/whitespace.txt" => replace_whitespace(concat!("my ", "value"))),
        "my value"
    );
    assert_eq!(
        include_str!("fixtures/message.txt" => trim => replace_whitespace("-")),
        "Hello,-world!"
    );
    assert_eq!(
        include_str!("fixtures/message.txt" => replace_whitespace("-") => trim),
        "-Hello,-world!-"
    );
    assert_eq!(
        include_str::include_lines!("fixtures/message.txt" => trim => replace_whitespace("\n")),
        &["Hello,", "world!"]
    );
}

#[test]
fn operations_compose_left_to_right() {
    const TEXT: &str = include_str!(
        "fixtures/message.txt" =>
        replace_whitespace(" ") =>
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
fn raw_inclusion_matches_builtin() {
    assert_eq!(
        include_str!("fixtures/message.txt",),
        core::include_str!("fixtures/message.txt")
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
        include_str!("fixtures/empty.txt" => trim => replace_whitespace(" ")),
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
        include_str::include_str!("fixtures/config.jsonc" => jsonc)
    );
}
