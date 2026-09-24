//! Implementation details for the exported macros; not a supported public API.

#[cfg(test)]
mod tests;

#[cfg(test)]
mod operations_tests;

#[cfg(test)]
mod edge_tests;

mod json;
mod transforms;
pub use json::{json, json_len, jsonc, jsonc_len};
pub use transforms::{replace, replace_len, strip_prefix, strip_prefix_len};

pub const fn as_str(bytes: &[u8]) -> &str {
    match core::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => panic!("include_str: invalid UTF-8 in processed output"),
    }
}

// The input originates from &str. Decode one scalar at a character boundary.
const fn whitespace_len(bytes: &[u8], i: usize) -> usize {
    let first = bytes[i];
    let (scalar, len) = if first < 128 {
        (first as u32, 1)
    } else if first < 224 {
        (((first & 31) as u32) << 6 | (bytes[i + 1] & 63) as u32, 2)
    } else if first < 240 {
        (
            ((first & 15) as u32) << 12
                | ((bytes[i + 1] & 63) as u32) << 6
                | (bytes[i + 2] & 63) as u32,
            3,
        )
    } else {
        // There are no whitespace scalars encoded with four bytes.
        return 0;
    };
    match scalar {
        0x09..=0x0d
        | 0x20
        | 0x85
        | 0xa0
        | 0x1680
        | 0x2000..=0x200a
        | 0x2028
        | 0x2029
        | 0x202f
        | 0x205f
        | 0x3000 => len,
        _ => 0,
    }
}

const fn char_len(first: u8) -> usize {
    if first < 128 {
        1
    } else if first < 224 {
        2
    } else if first < 240 {
        3
    } else {
        4
    }
}

const fn trim_bounds(input: &str) -> (usize, usize) {
    let bytes = input.as_bytes();
    let mut start = 0;
    while start < bytes.len() {
        let len = whitespace_len(bytes, start);
        if len == 0 {
            break;
        }
        start += len;
    }
    let mut end = start;
    let mut i = start;
    while i < bytes.len() {
        let white = whitespace_len(bytes, i);
        if white == 0 {
            i += char_len(bytes[i]);
            end = i;
        } else {
            i += white;
        }
    }
    (start, end)
}

pub const fn trim_len(input: &str) -> usize {
    let (start, end) = trim_bounds(input);
    end - start
}

pub const fn trim<const N: usize>(input: &str) -> [u8; N] {
    let (start, end) = trim_bounds(input);
    assert!(N == end - start);
    let mut output = [0; N];
    let mut i = 0;
    while i < N {
        output[i] = input.as_bytes()[start + i];
        i += 1;
    }
    output
}

// Share the sizing and emitting passes; copy trimmed content and line endings
// separately so trimming cannot remove blank lines or normalize CRLF to LF.
const fn scan_trim_lines<const N: usize>(input: &str, emit: bool) -> ([u8; N], usize) {
    let bytes = input.as_bytes();
    let mut output = [0; N];
    let mut written = 0;
    let mut i = 0;
    while i < bytes.len() {
        let mut start = i;
        let mut end = i;
        let mut leading = true;
        while i < bytes.len() && bytes[i] != b'\n' {
            let white = whitespace_len(bytes, i);
            let len = char_len(bytes[i]);
            if white == 0 {
                leading = false;
                end = i + len;
            } else if leading {
                start += len;
            }
            i += len;
        }
        while start < end {
            if emit {
                output[written] = bytes[start];
            }
            written += 1;
            start += 1;
        }
        if i < bytes.len() {
            if i > 0 && bytes[i - 1] == b'\r' {
                if emit {
                    output[written] = b'\r';
                }
                written += 1;
            }
            if emit {
                output[written] = b'\n';
            }
            written += 1;
            i += 1;
        }
    }
    (output, written)
}

pub const fn trim_lines_len(input: &str) -> usize {
    scan_trim_lines::<0>(input, false).1
}

pub const fn trim_lines<const N: usize>(input: &str) -> [u8; N] {
    let (output, written) = scan_trim_lines::<N>(input, true);
    assert!(N == written);
    output
}

const fn pair(bytes: &[u8], i: usize, a: u8, b: u8) -> bool {
    i + 1 < bytes.len() && bytes[i] == a && bytes[i + 1] == b
}

const fn identifier(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'$' | 128..=255)
}

const fn identifier_before(bytes: &[u8], end: usize) -> bool {
    if end == 0 {
        return false;
    }
    let mut start = end - 1;
    while bytes[start] & 0xc0 == 0x80 {
        start -= 1;
    }
    whitespace_len(bytes, start) == 0 && identifier(bytes[start])
}

const fn quoted_end(bytes: &[u8], start: usize, close: u8, escape: bool) -> usize {
    let mut i = start + 1;
    while i < bytes.len() {
        if escape && bytes[i] == b'\\' {
            i += 2;
        } else if bytes[i] == close {
            if i + 1 < bytes.len() && bytes[i + 1] == close {
                i += 2;
            } else {
                return i + 1;
            }
        } else {
            i += 1;
        }
    }
    panic!("include_sql_str!: unterminated quoted string or identifier");
}

// Returns zero when the dollar sign does not start a dollar-quoted string.
const fn dollar_end(bytes: &[u8], start: usize) -> usize {
    if identifier_before(bytes, start) {
        return 0;
    }
    let mut tag_end = start + 1;
    while tag_end < bytes.len() && bytes[tag_end] != b'$' {
        let b = bytes[tag_end];
        if !(matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'_' | 128..=255)
            || (tag_end > start + 1 && b.is_ascii_digit()))
        {
            return 0;
        }
        if whitespace_len(bytes, tag_end) > 0 {
            return 0;
        }
        tag_end += char_len(b);
    }
    if tag_end == bytes.len() {
        return 0;
    }
    let len = tag_end - start + 1;
    let mut i = tag_end + 1;
    while i + len <= bytes.len() {
        let mut j = 0;
        while j < len && bytes[i + j] == bytes[start + j] {
            j += 1;
        }
        if j == len {
            return i + len;
        }
        i += 1;
    }
    panic!("include_sql_str!: unterminated dollar-quoted string");
}

// A single scanner drives both the sizing and emitting passes so they agree.
const fn scan<const N: usize>(input: &str, emit: bool) -> ([u8; N], usize) {
    let bytes = input.as_bytes();
    let mut output = [0; N];
    let mut written = 0;
    let mut pending_space = false;
    let mut i = 0;
    while i < bytes.len() {
        let white = whitespace_len(bytes, i);
        if white > 0 {
            pending_space = true;
            i += white;
            continue;
        }
        if pair(bytes, i, b'-', b'-') {
            i += 2;
            while i < bytes.len() && bytes[i] != b'\n' && bytes[i] != b'\r' {
                i += 1;
            }
            pending_space = true;
            continue;
        }
        if pair(bytes, i, b'/', b'*') {
            let mut depth = 1;
            i += 2;
            while i < bytes.len() && depth > 0 {
                if pair(bytes, i, b'/', b'*') {
                    depth += 1;
                    i += 2;
                } else if pair(bytes, i, b'*', b'/') {
                    depth -= 1;
                    i += 2;
                } else {
                    i += 1;
                }
            }
            assert!(depth == 0, "include_sql_str!: unterminated block comment");
            pending_space = true;
            continue;
        }
        let end = match bytes[i] {
            b'\'' => {
                let escape = i > 0
                    && matches!(bytes[i - 1], b'E' | b'e')
                    && !identifier_before(bytes, i - 1);
                quoted_end(bytes, i, b'\'', escape)
            }
            b'"' | b'`' => quoted_end(bytes, i, bytes[i], false),
            b'[' => quoted_end(bytes, i, b']', false),
            b'$' => {
                let end = dollar_end(bytes, i);
                if end == 0 { i + 1 } else { end }
            }
            _ => i + char_len(bytes[i]),
        };
        if pending_space && written > 0 {
            if emit {
                output[written] = b' ';
            }
            written += 1;
        }
        pending_space = false;
        while i < end {
            if emit {
                output[written] = bytes[i];
            }
            written += 1;
            i += 1;
        }
    }
    (output, written)
}

pub const fn sql_len(input: &str) -> usize {
    scan::<0>(input, false).1
}

pub const fn sql<const N: usize>(input: &str) -> [u8; N] {
    let (output, written) = scan::<N>(input, true);
    assert!(N == written);
    output
}
