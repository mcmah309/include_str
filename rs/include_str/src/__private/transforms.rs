use super::char_len;

const fn matches_at(input: &[u8], at: usize, pattern: &[u8]) -> bool {
    if pattern.len() > input.len() - at {
        return false;
    }
    let mut i = 0;
    while i < pattern.len() {
        if input[at + i] != pattern[i] {
            return false;
        }
        i += 1;
    }
    true
}

pub(super) const fn scan_replace<const N: usize>(
    input: &str,
    from: &str,
    to: &str,
    emit: bool,
) -> ([u8; N], usize) {
    let bytes = input.as_bytes();
    let mut output = [0; N];
    let mut written = 0;
    let mut i = 0;
    loop {
        if matches_at(bytes, i, from.as_bytes()) {
            let mut j = 0;
            while j < to.len() {
                if emit {
                    output[written] = to.as_bytes()[j];
                }
                written += 1;
                j += 1;
            }
            if !from.is_empty() {
                i += from.len();
                continue;
            }
        }
        if i == bytes.len() {
            break;
        }
        let end = i + char_len(bytes[i]);
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

pub const fn replace_len(input: &str, from: &str, to: &str) -> usize {
    scan_replace::<0>(input, from, to, false).1
}

pub const fn replace<const N: usize>(input: &str, from: &str, to: &str) -> [u8; N] {
    let (output, written) = scan_replace::<N>(input, from, to, true);
    assert!(written == N);
    output
}

pub(super) const fn scan_strip_prefix<const N: usize>(
    input: &str,
    prefix: &str,
    emit: bool,
) -> ([u8; N], usize) {
    let bytes = input.as_bytes();
    let pattern = prefix.as_bytes();
    let mut j = 0;
    while j < pattern.len() {
        assert!(
            pattern[j] != b'\r' && pattern[j] != b'\n',
            "include_str_strip_line_prefix!: prefix must not contain a line ending"
        );
        j += 1;
    }
    let mut output = [0; N];
    let mut written = 0;
    let mut i = 0;
    let mut line_start = true;
    while i < bytes.len() {
        if line_start && matches_at(bytes, i, pattern) {
            i += pattern.len();
        }
        if i == bytes.len() {
            break;
        }
        line_start = bytes[i] == b'\n';
        if emit {
            output[written] = bytes[i];
        }
        written += 1;
        i += 1;
    }
    (output, written)
}

pub const fn strip_prefix_len(input: &str, prefix: &str) -> usize {
    scan_strip_prefix::<0>(input, prefix, false).1
}

pub const fn strip_prefix<const N: usize>(input: &str, prefix: &str) -> [u8; N] {
    let (output, written) = scan_strip_prefix::<N>(input, prefix, true);
    assert!(written == N);
    output
}
