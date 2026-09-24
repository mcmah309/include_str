# include_str

A dependency-free, `no_std` Rust crate for including UTF-8 files with compile-time
preprocessing:

- `include_str::include_str!`: Rust's built-in macro, re-exported.
- `include_str::include_str_trim!`: trim leading and trailing Unicode whitespace.
- `include_str::include_sql_str!`: strip SQL comments and compact whitespace while
  preserving quoted content.

See the [crate README](rs/include_str/README.md) for usage and SQL dialect details.

Run `cargo test --workspace` to test the crate.
