//! Cost test for unterminated block comments (`SPECIFICATION.md`
//! §6.6, issue #75).
//!
//! A `/*` with no `*/` is not a comment, and the scanner learns that
//! by reading to the end of input. It must do so once per parse, not
//! once per `/*`. Time is not measured: the parser reads its input
//! through a callback here, and the test counts the bytes it asks for.

use tree_sitter::Parser;

/// Lines in each input. The scanner read the whole input once per
/// line before the fix, so the ratio below was in the thousands.
const LINES: usize = 2_000;

/// Bytes handed to the parser per call of the read callback. Small
/// enough that a scan to the end of input needs many calls.
const CHUNK_BYTES: usize = 1_024;

/// Most bytes the parser may ask for, per byte of input. Measured: 4.9
/// for the plain series and 5.9 for the `record` one. One scan to the
/// end of input per line gives about 1000 and more.
const MAX_BYTES_SERVED_PER_INPUT_BYTE: usize = 20;

/// Parses `LINES` copies of `line` and returns the bytes served to the
/// parser divided by the input length, rounded down.
fn bytes_served_per_input_byte(line: &str) -> usize {
    let source = line.repeat(LINES);
    let bytes = source.as_bytes();
    let mut parser = Parser::new();
    parser
        .set_language(&dekobon_tree_sitter_groovy::LANGUAGE.into())
        .expect("load Groovy grammar");

    let mut served = 0_usize;
    let tree = parser
        .parse_with_options(
            &mut |offset, _| {
                let end = bytes.len().min(offset + CHUNK_BYTES);
                let chunk = &bytes[offset.min(end)..end];
                served += chunk.len();
                chunk
            },
            None,
            None,
        )
        .expect("parse");
    assert_eq!(tree.root_node().end_byte(), bytes.len());
    served / bytes.len()
}

#[test]
fn unterminated_comments_are_scanned_once() {
    let ratio = bytes_served_per_input_byte("rekord a /* x\n");
    assert!(
        ratio < MAX_BYTES_SERVED_PER_INPUT_BYTE,
        "parser read the input {ratio} times over",
    );
}

#[test]
fn unterminated_comments_in_record_headers_are_scanned_once() {
    let ratio = bytes_served_per_input_byte("record a /* x\n");
    assert!(
        ratio < MAX_BYTES_SERVED_PER_INPUT_BYTE,
        "parser read the input {ratio} times over",
    );
}
