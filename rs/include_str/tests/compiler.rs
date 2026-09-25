//! Exercise actual macro expansion in a separate consumer, including diagnostics.
//! Uses rustc directly to keep the test suite dependency-free and offline.
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    time::{SystemTime, UNIX_EPOCH},
};

struct Consumer(PathBuf);

impl Consumer {
    fn new() -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("include-str-tests-{}-{nonce}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        let consumer = Self(dir);
        let output = rustc()
            .args([
                "--edition=2024",
                "--crate-name=include_str",
                "--crate-type=rlib",
            ])
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"))
            .arg("-o")
            .arg(consumer.0.join("libinclude_str.rlib"))
            .output()
            .expect("rustc must be available to run compiler tests");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        consumer
    }

    fn compile(&self, source: &str) -> Output {
        fs::write(self.0.join("consumer.rs"), source).unwrap();
        rustc()
            .current_dir(&self.0)
            .args([
                "--edition=2024",
                "--crate-type=lib",
                "--emit=metadata,dep-info",
                "--extern",
                "strings=libinclude_str.rlib",
                "consumer.rs",
            ])
            .output()
            .unwrap()
    }

    fn write(&self, path: &str, bytes: impl AsRef<[u8]>) {
        fs::write(self.0.join(path), bytes).unwrap();
    }
}

impl Drop for Consumer {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn rustc() -> Command {
    Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
}

#[test]
fn public_macros_reject_malformed_sql_at_compile_time() {
    let consumer = Consumer::new();
    for (sql, diagnostic) in [
        ("/*", "unterminated block comment"),
        ("/* /* */", "unterminated block comment"),
        ("SELECT 1; /* 🦀", "unterminated block comment"),
        ("'", "unterminated quoted string or identifier"),
        ("'''", "unterminated quoted string or identifier"),
        ("SELECT '日本語", "unterminated quoted string or identifier"),
        ("\"x", "unterminated quoted string or identifier"),
        ("`x", "unterminated quoted string or identifier"),
        ("[x", "unterminated quoted string or identifier"),
        ("[]]", "unterminated quoted string or identifier"),
        ("E'\\", "unterminated quoted string or identifier"),
        ("E'\\'", "unterminated quoted string or identifier"),
        ("$$", "unterminated dollar-quoted string"),
        ("$tag$x", "unterminated dollar-quoted string"),
        ("$a$x$A$", "unterminated dollar-quoted string"),
        ("$é$🦀", "unterminated dollar-quoted string"),
    ] {
        consumer.write("input.sql", sql);
        // Even invocation in an ordinary function must be evaluated at compile time.
        let output = consumer
            .compile("pub fn query() -> &'static str { strings::include_sql_str!(\"input.sql\") }");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "malformed SQL compiled: {sql:?}");
        assert!(
            stderr.contains("E0080") && stderr.contains(diagnostic),
            "{sql:?}: {stderr}"
        );
        assert!(!stderr.contains("index out of bounds"), "{stderr}");
    }
}

#[test]
fn public_macros_reject_missing_files_and_invalid_utf8() {
    let consumer = Consumer::new();
    consumer.write("invalid.txt", [0xff, 0xfe, b'a']);
    for name in [
        "include_str",
        "include_lines",
        "include_str_trim",
        "include_str_trim_lines",
        "include_sql_str",
        "include_str_json",
        "include_str_jsonc",
        "include_str_replace",
        "include_str_strip_line_prefix",
    ] {
        for (file, diagnostic) in [("missing.txt", "couldn't read"), ("invalid.txt", "utf-8")] {
            let extra = match name {
                "include_str_replace" => ", \"a\", \"b\"",
                "include_str_strip_line_prefix" => ", \"a\"",
                _ => "",
            };
            let output = consumer.compile(&format!(
                "pub const VALUE: &{} = strings::{name}!(\"{file}\"{extra});",
                if name == "include_lines" {
                    "[&str]"
                } else {
                    "str"
                }
            ));
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!output.status.success(), "{name} accepted {file}");
            assert!(
                stderr.to_lowercase().contains(diagnostic),
                "{name}: {stderr}"
            );
        }
    }
}

#[test]
fn renamed_no_std_consumer_uses_static_results_and_tracks_files() {
    let consumer = Consumer::new();
    fs::create_dir(consumer.0.join("nested")).unwrap();
    consumer.write("nested/query.sql", "-- start\nSELECT /* x */ 'a  -- b';\n");
    consumer.write("nested/text.txt", "\u{3000}é 🦀\n  日本語\u{a0}");
    consumer.write("nested/config.json", " { \"a\" : [true, null, 1e99] } ");
    consumer.write(
        "nested/mod.rs",
        r#"
        pub const QUERY: &str = strings::include_sql_str!("query.sql",);
        pub static TEXT: &str = strings::include_str_trim!(concat!("text", ".txt"),);
        pub const RAW: &str = strings::include_str!("text.txt");
        pub const LINES: &str = strings::include_str_trim_lines!(concat!("text", ".txt"),);
        pub const REPLACED: &str = strings::include_str_replace!("text.txt", "🦀", "Rust");
        pub const STRIPPED: &str = strings::include_str_strip_line_prefix!("text.txt", "  ");
        pub const JSON: &str = strings::include_str_json!("config.json");
        pub const JSONC: &str = strings::include_str_jsonc!("config.json");
    "#,
    );
    let output = consumer.compile(
        r#"
        #![no_std]
        mod nested;
        const fn equal(a: &str, b: &str) -> bool {
            let (a, b) = (a.as_bytes(), b.as_bytes());
            if a.len() != b.len() { return false; }
            let mut i = 0;
            while i < a.len() {
                if a[i] != b[i] { return false; }
                i += 1;
            }
            true
        }
        const _: () = assert!(equal(nested::QUERY, "SELECT 'a  -- b';"));
        const _: () = assert!(equal(nested::TEXT, "é 🦀\n  日本語"));
        const _: () = assert!(equal(nested::LINES, "é 🦀\n日本語"));
        const _: () = assert!(equal(nested::REPLACED, "\u{3000}é Rust\n  日本語\u{a0}"));
        const _: () = assert!(equal(nested::STRIPPED, "\u{3000}é 🦀\n日本語\u{a0}"));
        const _: () = assert!(equal(nested::JSON, "{\"a\":[true,null,1e99]}"));
        const _: () = assert!(equal(nested::JSONC, nested::JSON));
        const _: () = assert!(equal(nested::RAW, "\u{3000}é 🦀\n  日本語\u{a0}"));
        pub fn query() -> &'static str { nested::QUERY }
    "#,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let dependencies = fs::read_to_string(consumer.0.join("consumer.d"))
        .unwrap()
        .replace('\\', "/");
    assert!(dependencies.contains("nested/query.sql"), "{dependencies}");
    assert!(dependencies.contains("nested/text.txt"), "{dependencies}");
    assert!(
        dependencies.contains("nested/config.json"),
        "{dependencies}"
    );
}

#[test]
fn json_and_invalid_prefixes_fail_at_compile_time() {
    let consumer = Consumer::new();
    for input in [
        "",
        " ",
        "[1,]",
        "{\"x\":}",
        "01",
        "1 2",
        "true false",
        "NaN",
        "/*x*/{}",
        "\"\\uZZZZ\"",
        "\"a\nb\"",
        "\u{feff}{}",
        "\u{a0}null",
    ] {
        consumer.write("input.json", input);
        let output = consumer.compile(
            "pub fn json() -> &'static str { strings::include_str_json!(\"input.json\") }",
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "accepted {input:?}");
        assert!(
            stderr.contains("E0080") && stderr.contains("include_str!: json:"),
            "{stderr}"
        );
    }
    consumer.write(
        "input.json",
        format!("{}0{}", "[".repeat(129), "]".repeat(129)),
    );
    let output =
        consumer.compile("pub const JSON: &str = strings::include_str_json!(\"input.json\");");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("nesting exceeds 128"));
    // The documented maximum must also succeed in actual constant evaluation.
    consumer.write(
        "input.json",
        format!("{}0{}", "[".repeat(128), "]".repeat(128)),
    );
    let output =
        consumer.compile("pub const JSON: &str = strings::include_str_json!(\"input.json\");");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = consumer.compile(
        "pub const TEXT: &str = strings::include_str_strip_line_prefix!(\"input.json\", \"\\n\");",
    );
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("prefix must not contain a line ending")
    );
}

#[test]
fn jsonc_validates_at_compile_time_and_accepts_only_documented_extensions() {
    let consumer = Consumer::new();
    for input in [
        "/*",
        "//only comment",
        "[1,,]",
        "{,}",
        "[1,/*",
        "[1/**/2]",
        "tr/**/ue",
        "null/**/null",
        "{a:1}",
        "{'a':1}",
        "[0xFF]",
        "\"\\x\"",
    ] {
        consumer.write("input.jsonc", input);
        let output = consumer.compile(
            "pub fn value() -> &'static str { strings::include_str_jsonc!(\"input.jsonc\") }",
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "accepted {input:?}");
        assert!(
            stderr.contains("E0080") && stderr.contains("include_str!: jsonc:"),
            "{stderr}"
        );
    }
    consumer.write("input.jsonc", "//start\n{\"a\":[true,/*last*/],}//end");
    let output = consumer.compile("#![no_std]\npub const JSON: &str = strings::include_str_jsonc!(\"input.jsonc\");\nconst _: () = assert!(JSON.len() == 12);");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let dependencies = fs::read_to_string(consumer.0.join("consumer.d")).unwrap();
    assert!(dependencies.contains("input.jsonc"));
}

#[test]
fn every_macro_works_in_all_contexts_with_renaming_and_shadowed_names() {
    let consumer = Consumer::new();
    consumer.write("input with spaces 🦀.txt", " \t\"é 🦀\" \r\n");
    let cases = [
        ("include_str", "", " \t\"é 🦀\" \r\n"),
        (
            "include_str",
            " => trim => replace(FROM, TO)",
            "\"Rust 🦀\"",
        ),
        (
            "include_str",
            " => strip_line_prefix(PREFIX) => collapse_whitespace(space) => trim",
            "\"é 🦀\"",
        ),
        ("include_str_trim", "", "\"é 🦀\""),
        ("include_str_trim_lines", "", "\"é 🦀\"\r\n"),
        ("include_sql_str", "", "\"é 🦀\""),
        ("include_str_json", "", "\"é 🦀\""),
        ("include_str_jsonc", "", "\"é 🦀\""),
        ("include_str_replace", ", FROM, TO", " \t\"Rust 🦀\" \r\n"),
        ("include_str_strip_line_prefix", ", PREFIX", "\"é 🦀\" \r\n"),
    ];
    let mut source = String::from(
        r#"
        #![no_std]
        const INPUT: &str = "caller";
        const TEXT: &str = "caller";
        const LEN: usize = 123;
        const BYTES: &[u8] = b"caller";
        const FROM: &str = "é";
        const TO: &str = "Rust";
        const PREFIX: &str = " \t";
        const fn equal(a: &str, b: &str) -> bool {
            let (a,b) = (a.as_bytes(), b.as_bytes());
            if a.len() != b.len() { return false; }
            let mut i = 0;
            while i < a.len() { if a[i] != b[i] { return false; } i += 1; }
            true
        }
    "#,
    );
    for (index, (name, extra, expected)) in cases.iter().enumerate() {
        // Trailing commas, concat!, a renamed dependency, repeated expansion,
        // expression/static/const contexts and caller constants named like internals.
        let invocation =
            format!("strings::{name}!(concat!(\"input with spaces \", \"🦀.txt\"){extra},)");
        source.push_str(&format!(
            "const C{index}: &str = {invocation};\nstatic S{index}: &str = {invocation};\n\
             pub fn f{index}() -> &'static str {{ {invocation} }}\n\
             const _: () = assert!(equal(C{index}, {expected:?}));\n\
             const _: () = assert!(equal(S{index}, {expected:?}));\n"
        ));
    }
    source.push_str("const _: () = assert!(equal(INPUT,TEXT) && LEN == 123 && BYTES.len() == 6);");
    let output = consumer.compile(&source);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn every_macro_rejects_invalid_arguments_without_runtime_fallback() {
    let consumer = Consumer::new();
    consumer.write("input.txt", "null");
    for name in [
        "include_str",
        "include_lines",
        "include_str_trim",
        "include_str_trim_lines",
        "include_sql_str",
        "include_str_replace",
        "include_str_strip_line_prefix",
        "include_str_json",
        "include_str_jsonc",
    ] {
        for arguments in ["", "\"input.txt\", \"x\", \"y\", \"extra\""] {
            let output = consumer.compile(&format!(
                "pub const X: &str = strings::{name}!({arguments});"
            ));
            assert!(!output.status.success(), "{name} accepted ({arguments})");
        }
    }
    for body in [
        "strings::include_str_replace!(\"input.txt\", runtime, \"x\")",
        "strings::include_str_replace!(\"input.txt\", \"x\", runtime)",
        "strings::include_str_strip_line_prefix!(\"input.txt\", runtime)",
        "strings::include_str_replace!(\"input.txt\", 42, \"x\")",
        "strings::include_str_strip_line_prefix!(\"input.txt\", b\"x\")",
        "strings::include_str!(\"input.txt\" => trim => replace(runtime, \"x\"))",
        "strings::include_str!(\"input.txt\" => replace(\"x\", runtime) => trim)",
        "strings::include_str!(\"input.txt\" => strip_line_prefix(runtime))",
    ] {
        let output = consumer.compile(&format!(
            "pub fn test(runtime: &str) -> &'static str {{ {body} }}"
        ));
        assert!(!output.status.success(), "accepted {body}");
    }
}

#[test]
fn pipelines_reject_old_and_malformed_separators() {
    let consumer = Consumer::new();
    consumer.write("input.txt", "hello");
    for arguments in [
        "\"input.txt\", trim",
        "\"input.txt\", trim, json",
        "\"input.txt\" => trim, json",
        "\"input.txt\", trim => json",
        "\"input.txt\" =>",
        "\"input.txt\" => trim =>",
        "\"input.txt\" -> trim",
        "\"input.txt\" |> trim",
    ] {
        let output = consumer.compile(&format!(
            "pub const TEXT: &str = strings::include_str!({arguments});"
        ));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "accepted {arguments}");
        assert!(
            stderr.contains("expected a path followed by operations separated by =>")
                || (arguments.contains("|>") && stderr.contains("expected expression")),
            "{arguments}: {stderr}"
        );
    }
}

#[test]
fn pipelines_reject_unknown_operations_and_preserve_validation() {
    let consumer = Consumer::new();
    consumer.write("input.txt", "/*");
    for (operations, diagnostic) in [
        (
            "trim => typo",
            "unknown operation or invalid arguments: typo",
        ),
        ("trim()", "unknown operation or invalid arguments: trim"),
        (
            "collapse_whitespace(tab)",
            "unknown operation or invalid arguments",
        ),
        (
            "replace(\"a\")",
            "unknown operation or invalid arguments: replace",
        ),
        (
            "strip_line_prefix()",
            "unknown operation or invalid arguments",
        ),
        ("trim => sql", "unterminated block comment"),
        ("trim => json", "E0080"),
        ("trim => jsonc", "E0080"),
        (
            "strip_line_prefix(\"\\n\")",
            "prefix must not contain a line ending",
        ),
    ] {
        let output = consumer.compile(&format!(
            "pub fn text() -> &'static str {{ strings::include_str!(\"input.txt\" => {operations}) }}"
        ));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "accepted {operations}");
        assert!(stderr.contains(diagnostic), "{operations}: {stderr}");
    }
}

// This quoted string is valid SQL, JSON and JSONC throughout every combination.
// Independent string operations provide the expected result for all 81 pairs.
const PIPELINE_OPERATIONS: [&str; 9] = [
    "trim",
    "trim_lines",
    "collapse_whitespace",
    "collapse_whitespace(space)",
    "replace(\"a\", \"aa\")",
    "strip_line_prefix(\" \")",
    "sql",
    "json",
    "jsonc",
];

fn reference_pipeline_step(input: &str, operation: usize) -> String {
    match operation {
        0 | 6..=8 => input.trim().into(),
        1 => input
            .split_inclusive('\n')
            .map(|line| {
                let (text, ending) = if let Some(text) = line.strip_suffix("\r\n") {
                    (text, "\r\n")
                } else if let Some(text) = line.strip_suffix('\n') {
                    (text, "\n")
                } else {
                    (line, "")
                };
                format!("{}{ending}", text.trim())
            })
            .collect(),
        2 | 3 => {
            let mut output = String::new();
            let mut chars = input.chars().peekable();
            while let Some(c) = chars.next() {
                if !c.is_whitespace() {
                    output.push(c);
                    continue;
                }
                let mut run = String::from(c);
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    run.push(chars.next().unwrap());
                }
                output.push(if operation == 3 {
                    ' '
                } else if run.contains(['\n', '\r']) {
                    '\n'
                } else if run.contains('\t') {
                    '\t'
                } else {
                    ' '
                });
            }
            output
        }
        4 => input.replace('a', "aa"),
        5 => input
            .split_inclusive('\n')
            .map(|line| line.strip_prefix(' ').unwrap_or(line))
            .collect(),
        _ => unreachable!(),
    }
}

#[test]
fn all_operation_pairs_produce_expected_text_in_a_renamed_no_std_consumer() {
    let consumer = Consumer::new();
    let input = " \t\"a  b\" \r\n";
    consumer.write("input.txt", input);
    let mut source = String::from(
        r#"
        #![no_std]
        const fn equal(a: &str, b: &str) -> bool {
            let (a, b) = (a.as_bytes(), b.as_bytes());
            if a.len() != b.len() { return false; }
            let mut i = 0;
            while i < a.len() { if a[i] != b[i] { return false; } i += 1; }
            true
        }
    "#,
    );
    for (a, first) in PIPELINE_OPERATIONS.iter().enumerate() {
        for (b, second) in PIPELINE_OPERATIONS.iter().enumerate() {
            let expected = reference_pipeline_step(&reference_pipeline_step(input, a), b);
            source.push_str(&format!(
                "const _: () = assert!(equal(strings::include_str!(\"input.txt\" => {first} => {second}), {expected:?}));\n"
            ));
        }
    }
    let output = consumer.compile(&source);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let dependencies = fs::read_to_string(consumer.0.join("consumer.d")).unwrap();
    assert!(dependencies.contains("input.txt"));
}

#[test]
fn validation_errors_identify_the_operation_at_every_pipeline_position() {
    let consumer = Consumer::new();
    consumer.write("input.txt", " \t\"a  b\" \r\n");
    // Make invalid text after any valid operation; the error must name the
    // validator even when another operation follows it, rather than reporting
    // an array sizing, UTF-8 or index error from a later stage.
    let corrupt = "replace(\"\\\"\", \"\") => replace(\"a\", \"/*\")";
    for operation in PIPELINE_OPERATIONS {
        for validator in ["sql", "json", "jsonc"] {
            for pipeline in [
                format!("{operation} => {corrupt} => {validator}"),
                format!("{corrupt} => {validator} => {operation}"),
            ] {
                let output = consumer.compile(&format!(
                    "pub fn text() -> &'static str {{ strings::include_str!(\"input.txt\" => {pipeline}) }}"
                ));
                let stderr = String::from_utf8_lossy(&output.stderr);
                assert!(!output.status.success(), "accepted {pipeline}");
                assert!(
                    stderr.contains(&format!("include_str!: {validator}:")),
                    "{pipeline}: {stderr}"
                );
                assert!(!stderr.contains("index out of bounds"), "{stderr}");
            }
        }
        for pipeline in [
            format!("typo => {operation}"),
            format!("{operation} => typo => trim"),
            format!("{operation} => typo"),
            format!("{operation} => strip_line_prefix(\"\\n\")"),
            format!("strip_line_prefix(\"\\n\") => {operation}"),
        ] {
            let output = consumer.compile(&format!(
                "pub const TEXT: &str = strings::include_str!(\"input.txt\" => {pipeline});"
            ));
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!output.status.success(), "accepted {pipeline}");
            let expected = if pipeline.contains("typo") {
                "unknown operation or invalid arguments: typo"
            } else {
                "include_str!: strip_line_prefix: prefix must not contain a line ending"
            };
            assert!(stderr.contains(expected), "{pipeline}: {stderr}");
        }
    }
}

#[test]
fn invalid_utf8_corpus_files_are_rejected_by_the_public_json_macros() {
    let consumer = Consumer::new();
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/json-test-suite/test_parsing");
    let mut tested = 0;
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let bytes = fs::read(&path).unwrap();
        if std::str::from_utf8(&bytes).is_ok() {
            continue;
        }
        consumer.write("input.json", bytes);
        for name in ["include_str_json", "include_str_jsonc"] {
            let output = consumer.compile(&format!(
                "pub const X: &str = strings::{name}!(\"input.json\");"
            ));
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(!output.status.success(), "{}: {name}", path.display());
            assert!(stderr.to_lowercase().contains("utf-8"), "{stderr}");
        }
        tested += 1;
    }
    assert!(
        tested >= 20,
        "invalid UTF-8 corpus coverage unexpectedly shrank: {tested}"
    );
}

#[test]
fn accepted_upstream_corpus_files_expand_at_compile_time() {
    let consumer = Consumer::new();
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/json-test-suite/test_parsing");
    let mut source = String::from("#![no_std]\n");
    let mut tested = 0;
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if !path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("y_")
        {
            continue;
        }
        source.push_str(&format!(
            "pub const JSON{tested}: &str = strings::include_str_json!({path:?});\n\
             pub static JSONC{tested}: &str = strings::include_str_jsonc!({path:?});\n"
        ));
        tested += 1;
    }
    assert!(tested > 90);
    let output = consumer.compile(&source);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn all_text_macros_preserve_raw_control_bytes_and_only_change_requested_content() {
    let consumer = Consumer::new();
    let mut input = String::new();
    for byte in 0..=127u8 {
        input.push(byte as char);
    }
    input.push_str("é🦀\u{feff}\u{200b}");
    consumer.write("input.txt", &input);
    let lines: String = input
        .split_inclusive('\n')
        .map(|line| {
            let (body, ending) = if let Some(body) = line.strip_suffix("\r\n") {
                (body, "\r\n")
            } else if let Some(body) = line.strip_suffix('\n') {
                (body, "\n")
            } else {
                (line, "")
            };
            format!("{}{ending}", body.trim())
        })
        .collect();
    let cases = [
        ("include_str", "", input.clone()),
        ("include_str_trim", "", input.trim().into()),
        ("include_str_trim_lines", "", lines),
        (
            "include_str_replace",
            ", \"\", \"🦀\"",
            input.replace("", "🦀"),
        ),
        (
            "include_str_replace",
            ", \"é\", \"\"",
            input.replace('é', ""),
        ),
        (
            "include_str_strip_line_prefix",
            ", \"\\0\"",
            input.strip_prefix('\0').unwrap().into(),
        ),
    ];
    let mut source = String::from(
        r#"
        #![no_std]
        const fn same(a: &str, b: &str) -> bool {
            let (a,b) = (a.as_bytes(), b.as_bytes());
            if a.len() != b.len() { return false; }
            let mut i = 0;
            while i < a.len() { if a[i] != b[i] { return false; } i += 1; }
            true
        }
    "#,
    );
    for (name, extra, expected) in cases {
        source.push_str(&format!(
            "const _: () = assert!(same(strings::{name}!(\"input.txt\"{extra}), {expected:?}));\n"
        ));
    }
    let output = consumer.compile(&source);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn large_documents_use_exact_sized_output_arrays_at_compile_time() {
    let consumer = Consumer::new();
    let mut input = String::from("{\n\"text\": \"");
    input.push_str(&"é 🦀 // literal ".repeat(128));
    input.push_str("\",\n\"values\": [");
    input.push_str(&" {\"x\": [true, false, null, 1.230e+9]},\n".repeat(127));
    input.push_str("{\"x\": [true, false, null, 1.230e+9]}] }");
    consumer.write("large.json", &input);
    consumer.write("large.jsonc", format!("/*start*/{input}//end"));
    let output = consumer.compile(
        r#"
        #![no_std]
        pub const JSON: &str = strings::include_str_json!("large.json");
        pub const JSONC: &str = strings::include_str_jsonc!("large.jsonc");
        const fn equal(a: &str, b: &str) -> bool {
            let (a,b) = (a.as_bytes(), b.as_bytes());
            if a.len() != b.len() { return false; }
            let mut i = 0;
            while i < a.len() { if a[i] != b[i] { return false; } i += 1; }
            true
        }
        const _: () = assert!(equal(JSON, JSONC));
        const _: () = assert!(JSON.len() > 5000);
    "#,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn empty_and_whitespace_files_have_explicit_results_for_every_macro() {
    let consumer = Consumer::new();
    let text_cases = [
        ("include_str", "", ""),
        ("include_str_trim", "", ""),
        ("include_str_trim_lines", "", ""),
        ("include_sql_str", "", ""),
        ("include_str_replace", ", \"\", \"x\"", "x"),
        ("include_str_strip_line_prefix", ", \"\"", ""),
    ];
    consumer.write("empty.txt", "");
    for (name, extra, expected) in text_cases {
        let output = consumer.compile(&format!(
            "pub const X: &str = strings::{name}!(\"empty.txt\"{extra});\nconst _: () = assert!(X.len() == {});",
            expected.len()
        ));
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    for content in ["", " \r\n\t", "\u{3000}\u{a0}"] {
        consumer.write("empty.txt", content);
        for name in ["include_str_json", "include_str_jsonc"] {
            let output = consumer.compile(&format!(
                "pub const X: &str = strings::{name}!(\"empty.txt\");"
            ));
            assert!(!output.status.success(), "{name} accepted {content:?}");
            assert!(String::from_utf8_lossy(&output.stderr).contains("E0080"));
        }
    }
}

#[test]
fn line_tables_work_in_renamed_no_std_consumers_and_track_the_source_file() {
    let consumer = Consumer::new();
    fs::create_dir(consumer.0.join("nested")).unwrap();
    consumer.write("nested/words 🦀.txt", "apple\r\n\nbanana\napple");
    consumer.write("nested/mod.rs", r#"
        pub const INPUT: &str = "caller";
        pub const LEN: usize = 99;
        pub const LINES: &[&str] = &["caller"];
        pub const RESULT: &str = "caller";
        pub const TABLE: &[&str] = strings::include_lines!(concat!("words ", "🦀.txt"),);
        pub static STATIC_TABLE: &[&str] = strings::include_lines!("words 🦀.txt");
        pub fn table() -> &'static [&'static str] { strings::include_lines!("words 🦀.txt") }
        pub const FROM: &str = "apple";
        pub const PROCESSED: &[&str] = strings::include_lines!(
            concat!("words ", "🦀.txt") => replace(FROM, " pear ") => trim_lines,
        );
        pub static PROCESSED_STATIC: &[&str] = strings::include_lines!(
            "words 🦀.txt" => trim_lines => replace(FROM, "pear")
        );
        pub fn processed() -> &'static [&'static str] {
            strings::include_lines!("words 🦀.txt" => replace(FROM, "pear"))
        }
        const _: () = assert!(PROCESSED.len() == 4 && PROCESSED[0].len() == 4);
        const _: () = assert!(PROCESSED[1].is_empty() && PROCESSED_STATIC[3].len() == 4);
        const _: () = assert!(TABLE.len() == 4 && STATIC_TABLE.len() == 4);
        const _: () = assert!(TABLE[0].len() == 5 && TABLE[1].is_empty() && TABLE[2].len() == 6);
        const _: () = assert!(INPUT.len() == 6 && LEN == 99 && LINES.len() == 1 && RESULT.len() == 6);
    "#);
    let output = consumer.compile("#![no_std]\nmod nested;\npub use nested::table;");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let dependencies = fs::read_to_string(consumer.0.join("consumer.d")).unwrap();
    assert!(dependencies.contains("words"), "{dependencies}");
    assert!(dependencies.contains("🦀.txt"), "{dependencies}");
}

#[test]
fn line_pipelines_reject_invalid_operations_and_input_at_compile_time() {
    let consumer = Consumer::new();
    consumer.write("input.txt", "not json");
    for (arguments, diagnostic) in [
        (r#""input.txt" => json"#, "include_str!: json:"),
        (
            r#""input.txt" => typo"#,
            "unknown operation or invalid arguments",
        ),
        (
            r#""input.txt" => replace("a")"#,
            "unknown operation or invalid arguments",
        ),
        (r#""input.txt", trim"#, "operations separated by =>"),
        (r#""input.txt" => trim =>"#, "operations separated by =>"),
        (
            r#""input.txt" => replace(runtime, "x")"#,
            "non-constant value",
        ),
    ] {
        let output = consumer.compile(&format!(
            "pub fn lines(runtime: &str) -> &'static [&'static str] {{ strings::include_lines!({arguments}) }}"
        ));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success(), "accepted {arguments}");
        assert!(stderr.contains(diagnostic), "{arguments}: {stderr}");
    }
}
