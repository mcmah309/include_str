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
        "include_str_trim",
        "include_str_trim_lines",
        "include_sql_str",
    ] {
        for (file, diagnostic) in [("missing.txt", "couldn't read"), ("invalid.txt", "utf-8")] {
            let output = consumer.compile(&format!(
                "pub const VALUE: &str = strings::{name}!(\"{file}\");"
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
    consumer.write(
        "nested/mod.rs",
        r#"
        pub const QUERY: &str = strings::include_sql_str!("query.sql",);
        pub static TEXT: &str = strings::include_str_trim!(concat!("text", ".txt"),);
        pub const RAW: &str = strings::include_str!("text.txt");
        pub const LINES: &str = strings::include_str_trim_lines!(concat!("text", ".txt"),);
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
}
