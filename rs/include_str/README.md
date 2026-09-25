# include_str

Include UTF-8 files as static strings, with optional compile-time preprocessing.
No dependencies, no allocation, no runtime processing, and compatible with `no_std`.

```rust
const RAW: &str = include_str::include_str!("message.txt");
const CONFIG: &str = include_str::include_str!(
    "config.jsonc" => replace("{{name}}", "Rust") => jsonc
);
const WORDS: &[&str] = include_str::include_lines!("words.txt" => trim_lines);
```

Separate operations with `=>`. They run **left to right**, each receiving the
previous step's text. Reorder or repeat operations as needed; every step must
accept its input or compilation fails. The result is always a `&'static str`.
With no operations, the file is included unchanged.

Order matters: `replace(...) => jsonc` validates and minifies the replaced text;
`jsonc => replace(...)` replaces text after validation, so the final result may
no longer be valid JSON.

| Operation | Effect |
| --- | --- |
| `trim` | Remove leading and trailing Unicode whitespace. |
| `trim_lines` | Trim each line, preserving LF/CRLF endings. |
| `collapse_whitespace` | Collapse each whitespace run using newline > tab > space precedence. |
| `replace_whitespace(replacement)` | Replace each whitespace run with a string, e.g. `" "`, `" / "`, or `""`. |
| `replace(from, to)` | Replace all non-overlapping literal matches. |
| `strip_line_prefix(prefix)` | Remove one matching prefix from each line, preserving line endings. |
| `strip_line_suffix(suffix)` | Remove one matching suffix from each line, preserving line endings. |
| `sql` | Strip SQL comments, compact unquoted whitespace, and trim. |
| `json` | Validate strict JSON and remove whitespace outside strings. |
| `jsonc` | Accept comments and trailing commas, producing minified JSON. |

Paths are relative to the invoking Rust source file. Path expressions such as
`concat!(env!("CARGO_MANIFEST_DIR"), "/queries/users.sql")` also work.
Files must exist and contain valid UTF-8. Rust tracks included files so changing
one triggers recompilation.
