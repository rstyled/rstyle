//! Token-selector resolution per the fixture encoding.
//!
//! A selector is `[exact Rust token sequence, 1-based occurrence]` among
//! matching contiguous token sequences in source order. Whitespace between
//! tokens is ignored for matching, but the resolved span includes it.
//! Overlapping selectors are legal. Comments and string contents are not
//! tokenized as identifier occurrences. All offsets are half-open zero-based
//! UTF-8 byte offsets into the exact source bytes.
//!
//! The lexer is written as pure cursor functions (`source`, `pos` in, new
//! `pos` out) so occurrence counting stays total and panic-free on
//! well-formed inputs.

/// One lexed token with its half-open byte span into the source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// Token text as it appears in the source (no whitespace normalization).
    pub text: String,
    /// Byte offset of the first token byte.
    pub start: usize,
    /// One past the last token byte.
    pub end: usize,
}

/// Operators matched with maximal munch, longest first.
const THREE_CHAR_OPS: [&str; 2] = ["<<=", ">>="];
const TWO_CHAR_OPS: [&str; 20] = [
    "==", "!=", "<=", ">=", "&&", "||", "<<", ">>", "+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=",
    "->", "=>", "::", "..",
];

/// Skip with the first succeeding skipper in order.
fn first_some(
    source: &str,
    pos: usize,
    skippers: &[fn(&str, usize) -> Option<usize>],
) -> Option<usize> {
    skippers.iter().find_map(|skip| skip(source, pos))
}

/// Skip one trivia run (whitespace, comment or string content) at `pos`.
fn skip_one(source: &str, pos: usize) -> Option<usize> {
    first_some(source, pos, &[skip_whitespace, skip_comment, skip_string])
}

/// Advance past all trivia starting at `pos`.
fn skip_trivia(source: &str, pos: usize) -> usize {
    let mut cursor = pos;

    while let Some(next) = skip_one(source, cursor) {
        cursor = next;
    }

    cursor
}

fn skip_whitespace(source: &str, pos: usize) -> Option<usize> {
    let head = source.get(pos..)?.chars().next()?;

    head.is_whitespace().then(|| pos + head.len_utf8())
}

fn skip_comment(source: &str, pos: usize) -> Option<usize> {
    skip_line_comment(source, pos).or_else(|| skip_block_comment(source, pos))
}

fn skip_line_comment(source: &str, pos: usize) -> Option<usize> {
    source.get(pos..)?.strip_prefix("//").map(|after| {
        let tail = after.find('\n').unwrap_or(after.len());
        pos + "//".len() + tail
    })
}

fn skip_block_comment(source: &str, pos: usize) -> Option<usize> {
    source.get(pos..)?.strip_prefix("/*")?;
    scan_block_close(source, pos + "/*".len(), 1)
}

/// One nested-comment scan step: bytes consumed and depth change.
fn block_step(rest: &str) -> Option<(usize, isize)> {
    if rest.starts_with("/*") {
        Some(("/*".len(), 1))
    } else if rest.starts_with("*/") {
        Some(("*/".len(), -1))
    } else {
        rest.chars().next().map(|next| (next.len_utf8(), 0))
    }
}

fn scan_block_close(source: &str, pos: usize, depth: usize) -> Option<usize> {
    let (mut cursor, mut depth) = (pos, depth);

    while depth > 0 {
        let (step, delta) = block_step(source.get(cursor..)?)?;
        cursor = cursor.saturating_add(step);
        depth = depth.saturating_add_signed(delta);
    }

    Some(cursor)
}

fn skip_string(source: &str, pos: usize) -> Option<usize> {
    first_some(
        source,
        pos,
        &[
            skip_quoted_string,
            skip_raw_string,
            skip_prefixed_string,
            skip_char_literal,
        ],
    )
}

fn skip_quoted_string(source: &str, pos: usize) -> Option<usize> {
    source.get(pos..)?.strip_prefix('"')?;
    scan_quoted_close(source, pos + 1, '"')
}

/// Bytes consumed by the character at `cursor` inside a quoted literal,
/// following a backslash escape when one is present.
fn quoted_char_len(source: &str, cursor: usize, head: char) -> Option<usize> {
    if head != '\\' {
        return Some(head.len_utf8());
    }

    let rest = source.get(cursor + head.len_utf8()..)?;
    Some(head.len_utf8() + rest.chars().next()?.len_utf8())
}

fn scan_quoted_close(source: &str, pos: usize, quote: char) -> Option<usize> {
    let mut cursor = pos;

    while let Some(head) = source.get(cursor..)?.chars().next() {
        if head == quote {
            return Some(cursor + head.len_utf8());
        }

        cursor += quoted_char_len(source, cursor, head)?;
    }

    None
}

fn skip_raw_string(source: &str, pos: usize) -> Option<usize> {
    let rest = source.get(pos..)?.strip_prefix('r')?;
    let hashes = rest.chars().take_while(|next| *next == '#').count();
    rest.get(hashes..)?.strip_prefix('"')?;

    let offset = pos + 'r'.len_utf8() + hashes + '"'.len_utf8();
    let closer: String = std::iter::once('"')
        .chain(std::iter::repeat_n('#', hashes))
        .collect();
    source
        .get(offset..)?
        .find(&closer)
        .map(|index| offset + index + closer.len())
}

fn skip_prefixed_string(source: &str, pos: usize) -> Option<usize> {
    let rest = source.get(pos..)?;
    let head = rest.chars().next()?;

    if head != 'b' && head != 'c' {
        return None;
    }

    rest.get(head.len_utf8()..)?.strip_prefix('"')?;
    scan_quoted_close(source, pos + head.len_utf8() + '"'.len_utf8(), '"')
}

fn skip_char_literal(source: &str, pos: usize) -> Option<usize> {
    char_literal_len(source.get(pos..)?).map(|len| pos + len)
}

/// Total length of a char literal starting at its opening quote.
fn char_literal_len(rest: &str) -> Option<usize> {
    let body = rest.strip_prefix('\'')?;
    let first = body.chars().next()?;

    if first == '\\' {
        let inner = escape_len(&body[first.len_utf8()..])?;
        Some('\''.len_utf8() + first.len_utf8() + inner)
    } else if first == '\'' || first == '\n' {
        None
    } else {
        body.get(first.len_utf8()..)?.strip_prefix('\'')?;
        Some('\''.len_utf8() + first.len_utf8() + '\''.len_utf8())
    }
}

/// Length after the backslash of a char escape, including the closing quote.
fn escape_len(rest: &str) -> Option<usize> {
    let second = rest.chars().next()?;

    if second != 'u' {
        rest.get(second.len_utf8()..)?.strip_prefix('\'')?;
        return Some(second.len_utf8() + '\''.len_utf8());
    }

    unicode_escape_len(&rest[second.len_utf8()..])
}

/// Length after `\u` of a unicode escape (`{hex}'` with the closing quote).
fn unicode_escape_len(rest: &str) -> Option<usize> {
    let digits = rest.strip_prefix('{')?;
    let close = digits.find('}')?;
    let body = digits.get(..close)?;

    if body.is_empty()
        || !body
            .chars()
            .all(|next| next.is_ascii_hexdigit() || next == '_')
    {
        return None;
    }

    digits.get(close + '}'.len_utf8()..)?.strip_prefix('\'')?;
    Some('{'.len_utf8() + close + '}'.len_utf8() + '\''.len_utf8())
}

fn is_ident_start(next: char) -> bool {
    next == '_' || next.is_alphabetic()
}

fn is_ident_continue(next: char) -> bool {
    next == '_' || next.is_alphanumeric()
}

/// Byte length of the leading character run satisfying `continues`.
fn leading_run_len(rest: &str, continues: impl Fn(char) -> bool) -> usize {
    rest.chars()
        .take_while(|next| continues(*next))
        .map(|next| next.len_utf8())
        .sum()
}

/// Leading alphanumeric run (digits, suffix letters, underscores).
fn leading_word_len(rest: &str) -> usize {
    leading_run_len(rest, |next| next.is_ascii_alphanumeric() || next == '_')
}

/// Whether `after` (text following the integer head) continues a float.
fn continues_float(after: &str) -> bool {
    after.strip_prefix('.').is_some_and(|tail| {
        tail.chars()
            .next()
            .is_some_and(|next| next.is_ascii_digit())
    })
}

/// Byte length of the number literal starting at `rest` (ASCII digit first).
/// A `.` joins the literal only for `d.d` floats, never for `..` ranges.
fn number_len(rest: &str) -> usize {
    let head = leading_word_len(rest);
    let after = rest.get(head..).unwrap_or("");

    if continues_float(after) {
        head + '.'.len_utf8() + leading_float_tail_len(&after['.'.len_utf8()..])
    } else {
        head
    }
}

/// Alphanumeric float tail after the decimal point (exponent letters share
/// the integer word class; signs stay separate tokens).
fn leading_float_tail_len(rest: &str) -> usize {
    leading_run_len(rest, |next| {
        next.is_ascii_alphanumeric() || next == '_' || next == '.'
    })
}

/// Byte length of the identifier starting at `rest` (identifier start first).
fn ident_len(rest: &str) -> usize {
    leading_run_len(rest, is_ident_continue)
}

fn match_fixed(rest: &str, candidates: &[&str]) -> Option<usize> {
    candidates
        .iter()
        .find(|op| rest.starts_with(*op))
        .map(|op| op.len())
}

/// Byte length of the token starting at `rest` (non-trivia head first).
fn token_len(rest: &str, head: char) -> usize {
    if is_ident_start(head) {
        ident_len(rest)
    } else if head.is_ascii_digit() {
        number_len(rest)
    } else {
        match_fixed(rest, &THREE_CHAR_OPS)
            .or_else(|| match_fixed(rest, &TWO_CHAR_OPS))
            .unwrap_or_else(|| head.len_utf8())
    }
}

/// Lex the next token at or after `pos`, returning it with the next cursor.
fn next_token(source: &str, pos: usize) -> Option<(Token, usize)> {
    let start = skip_trivia(source, pos);
    let rest = source.get(start..)?;
    let head = rest.chars().next()?;
    let len = token_len(rest, head);

    let token = Token {
        text: rest[..len].to_string(),
        start,
        end: start + len,
    };

    Some((token, start + len))
}

/// Lex the source into tokens, dropping whitespace, comments and literals
/// with uninspectable contents (strings). Punctuation uses maximal munch.
pub fn lex(source: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut pos = 0;

    while let Some((token, next)) = next_token(source, pos) {
        tokens.push(token);
        pos = next;
    }

    tokens
}

/// Split selector text into expected token spellings with the same lexer.
/// Whitespace in the selector is insignificant; punctuation adjacency (such
/// as `g(a)` or `&i32`) tokenizes exactly like source text.
fn selector_tokens(selector: &str) -> Vec<String> {
    lex(selector).into_iter().map(|token| token.text).collect()
}

/// All spans where the token run matches the selector, in source order.
pub fn resolve_all(source: &str, selector: &str) -> Vec<(usize, usize)> {
    let wanted = selector_tokens(selector);

    if wanted.is_empty() {
        return Vec::new();
    }

    let tokens = lex(source);
    let mut spans = Vec::new();
    let mut index = 0;

    while index + wanted.len() <= tokens.len() {
        let matches = tokens[index..index + wanted.len()]
            .iter()
            .zip(wanted.iter())
            .all(|(token, want)| token.text == *want);

        if matches {
            spans.push((tokens[index].start, tokens[index + wanted.len() - 1].end));
        }

        index += 1;
    }

    spans
}

/// Resolve the 1-based occurrence of a selector to its byte span.
/// Returns `None` for empty selectors, occurrence zero, or a missing match.
pub fn resolve(source: &str, selector: &str, occurrence: usize) -> Option<(usize, usize)> {
    if occurrence == 0 {
        return None;
    }

    resolve_all(source, selector)
        .into_iter()
        .nth(occurrence - 1)
}

/// Count matching token runs for a selector (occurrence-space size).
pub fn count_matches(source: &str, selector: &str) -> usize {
    resolve_all(source, selector).len()
}
