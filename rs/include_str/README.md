# include_str

Include UTF-8 files as static strings, with optional compile-time preprocessing.
No dependencies, no allocation, no runtime processing, and compatible with `no_std`.
Requires Rust 1.85 or newer.

```toml
[dependencies]
include_str = "0.0.1"
```

```rust
const RAW: &str = include_str::include_str!("message.txt");
const MESSAGE: &str = include_str::include_str_trim!("message.txt");
const LINES: &str = include_str::include_str_trim_lines!("message.txt");
const QUERY: &str = include_str::include_sql_str!("query.sql");
```

- `include_str!` is a direct re-export of Rust's built-in macro.
- `include_str_trim!` strips leading and trailing Unicode whitespace, like `str::trim`.
- `include_str_trim_lines!` trims Unicode whitespace from each line, preserving internal whitespace, blank lines, and LF/CRLF line endings (including the final one). A lone carriage return is whitespace, not a line separator.
- `include_sql_str!` removes SQL comments, collapses unquoted Unicode whitespace to single spaces, and trims the result.

For example, this SQL:

```sql
-- Find active users
SELECT  id, name
FROM /* table */ users
WHERE active = 1;
```

becomes `SELECT id, name FROM users WHERE active = 1;`.

Paths are relative to the invoking Rust source file. Path expressions such as
`concat!(env!("CARGO_MANIFEST_DIR"), "/queries/users.sql")` also work.
Files must exist and contain valid UTF-8. Rust tracks included files so changing
one triggers recompilation.
