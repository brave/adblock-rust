//! Common utilities used by the library. Some tests and benchmarks rely on this module having
//! public visibility.

#[cfg(target_pointer_width = "64")]
use seahash::hash;
#[cfg(target_pointer_width = "32")]
use seahash::reference::hash;

pub use arrayvec::ArrayVec;

pub type Hash = u64;

// A smaller version of Hash that is used in serialized format.
// Shouldn't be used to compare strings with each other.
pub type ShortHash = u32;

#[inline]
pub fn fast_hash(input: &str) -> Hash {
    hash(input.as_bytes()) as Hash
}

#[inline]
pub fn to_short_hash(hash: Hash) -> ShortHash {
    hash as ShortHash
}

#[inline]
fn is_allowed_filter(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '%'
}

/// A fixed-size array-like vector of hashes with maximum capacity of 256.
/// Used instread of Vec<Hash> to avoid heap allocations.
pub(crate) type TokensBuffer = ArrayVec<Hash, 256>;

fn fast_tokenizer_no_regex(
    pattern: &str,
    is_allowed_code: &dyn Fn(char) -> bool,
    skip_first_token: bool,
    skip_last_token: bool,
    tokens_buffer: &mut TokensBuffer,
) {
    // let mut tokens_buffer_index = 0;
    let mut inside: bool = false;
    let mut start = 0;
    let mut preceding_ch: Option<char> = None; // Used to check if a '*' is not just before a token

    for (i, c) in pattern.char_indices() {
        if tokens_buffer.capacity() - tokens_buffer.len() <= 1 {
            return; // reserve one free slot for the zero token
        }
        if is_allowed_code(c) {
            if !inside {
                inside = true;
                start = i;
            }
        } else if inside {
            inside = false;
            // Should not be followed by '*'
            if (start != 0 || !skip_first_token)
                && i - start > 1
                && c != '*'
                && preceding_ch != Some('*')
            {
                let hash = fast_hash(&pattern[start..i]);
                tokens_buffer.push(hash);
            }
            preceding_ch = Some(c);
        } else {
            preceding_ch = Some(c);
        }
    }

    if !skip_last_token && inside && pattern.len() - start > 1 && (preceding_ch != Some('*')) {
        let hash = fast_hash(&pattern[start..]);
        tokens_buffer.push(hash);
    }
}

pub(crate) fn tokenize_pooled(pattern: &str, tokens_buffer: &mut TokensBuffer) {
    fast_tokenizer_no_regex(pattern, &is_allowed_filter, false, false, tokens_buffer);
}

pub fn tokenize(pattern: &str) -> Vec<Hash> {
    let mut tokens_buffer = TokensBuffer::default();
    tokenize_to(pattern, &mut tokens_buffer);
    tokens_buffer.into_iter().collect()
}

pub(crate) fn tokenize_to(pattern: &str, tokens_buffer: &mut TokensBuffer) {
    fast_tokenizer_no_regex(pattern, &is_allowed_filter, false, false, tokens_buffer);
}

#[cfg(test)]
pub(crate) fn tokenize_filter(
    pattern: &str,
    skip_first_token: bool,
    skip_last_token: bool,
) -> Vec<Hash> {
    let mut tokens_buffer = TokensBuffer::default();
    tokenize_filter_to(
        pattern,
        skip_first_token,
        skip_last_token,
        &mut tokens_buffer,
    );
    tokens_buffer.into_iter().collect()
}

pub(crate) fn tokenize_filter_to(
    pattern: &str,
    skip_first_token: bool,
    skip_last_token: bool,
    tokens_buffer: &mut TokensBuffer,
) {
    fast_tokenizer_no_regex(
        pattern,
        &is_allowed_filter,
        skip_first_token,
        skip_last_token,
        tokens_buffer,
    );
}

/// Whether an escape sequence `\<ch>` is guaranteed to leave a non-token character
/// (or a string edge) at that position: either a zero-width anchor, or an escaped
/// literal that cannot be part of a token (`\.`, `\/`, `\?`, ...).
fn escape_guarantees_boundary(ch: char) -> bool {
    matches!(ch, 'b' | 'A' | 'z' | 'Z') || !is_allowed_filter(ch)
}

/// Consumes the remainder of a character class, assuming the opening `[` was
/// already consumed.
fn skip_regex_class(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) {
    // A `]` right after `[` or `[^` is a literal, not the end of the class.
    let mut at_start = true;
    while let Some((_, ch)) = chars.next() {
        match ch {
            '\\' => {
                chars.next();
            }
            '^' if at_start => continue,
            ']' if !at_start => return,
            _ => {}
        }
        at_start = false;
    }
}

/// Consumes the remainder of a group, assuming the opening `(` was already
/// consumed.
fn skip_regex_group(chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>) {
    let mut depth = 1usize;
    while let Some((_, ch)) = chars.next() {
        match ch {
            '\\' => {
                chars.next();
            }
            '[' => skip_regex_class(chars),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return;
                }
            }
            _ => {}
        }
    }
}

/// Reports the pending literal run, but only if it is delimited on both sides, i.e.
/// if a matching URL is guaranteed to tokenize it as a whole token.
fn flush_regex_run(
    run: &mut Option<(usize, usize)>,
    left_delimited: bool,
    right_delimited: bool,
    regex: &str,
    on_literal: &mut impl FnMut(&str),
) {
    if let Some((start, end)) = run.take()
        && left_delimited
        && right_delimited
        && end - start > 1
    {
        on_literal(&regex[start..end]);
    }
}

/// Reports every literal run of a complete regex (`/…/`) that is guaranteed to
/// appear verbatim, as a whole token, in *every* URL the regex can match.
///
/// A run is only reported when it is both mandatory (not quantified, not inside a
/// group, not part of an alternation) and delimited on both sides by something that
/// guarantees a non-token character in the URL, which is exactly the condition for
/// [`tokenize_pooled`] to produce that run as a token of a matching URL. Anything
/// that isn't trivially provable is skipped.
///
/// Returns `false` if the pattern turned out to be unanalyzable, in which case the
/// literals reported so far must be discarded.
fn scan_complete_regex_literals(pattern: &str, on_literal: &mut impl FnMut(&str)) -> bool {
    // Strip the enclosing `/` delimiters, mirroring `compile_regex`.
    let regex = pattern
        .strip_prefix('/')
        .and_then(|p| p.strip_suffix('/'))
        .unwrap_or(pattern);

    let mut chars = regex.char_indices().peekable();
    // Whether the character preceding the current position in a matching URL is
    // guaranteed not to be a token character. A regex without a leading `^` can
    // start matching in the middle of a URL word, so this starts out `false`.
    let mut boundary = false;
    let mut run: Option<(usize, usize)> = None;
    let mut run_left_delimited = false;

    while let Some((i, ch)) = chars.next() {
        match ch {
            // Alternation: only tokens common to every branch would be guaranteed,
            // which isn't worth analyzing. Give up on the whole pattern.
            '|' => return false,
            '\\' => {
                let escaped = chars.next().map(|(_, c)| c);
                let delimits = escaped.is_some_and(escape_guarantees_boundary)
                    // An optional or repeated atom delimits nothing.
                    && !matches!(chars.peek(), Some((_, '?' | '*' | '{')));
                flush_regex_run(&mut run, run_left_delimited, delimits, regex, on_literal);
                boundary = delimits;
            }
            '(' => {
                flush_regex_run(&mut run, run_left_delimited, false, regex, on_literal);
                skip_regex_group(&mut chars);
                boundary = false;
            }
            '[' => {
                flush_regex_run(&mut run, run_left_delimited, false, regex, on_literal);
                skip_regex_class(&mut chars);
                boundary = false;
            }
            // Zero-width anchors: the string edge is a token boundary.
            '^' | '$' => {
                flush_regex_run(&mut run, run_left_delimited, true, regex, on_literal);
                boundary = true;
            }
            // A quantifier here applies to the last character of the pending run,
            // making that character optional or repeatable, so the run is unusable.
            '?' | '*' | '+' | '{' => {
                run = None;
                if ch == '{' {
                    for (_, c) in chars.by_ref() {
                        if c == '}' {
                            break;
                        }
                    }
                }
                boundary = false;
            }
            // Matches any character, including token characters.
            '.' => {
                flush_regex_run(&mut run, run_left_delimited, false, regex, on_literal);
                boundary = false;
            }
            _ if is_allowed_filter(ch) => match &mut run {
                Some((_, end)) => *end = i + ch.len_utf8(),
                None => {
                    run = Some((i, i + ch.len_utf8()));
                    run_left_delimited = boundary;
                }
            },
            // Any other literal character is a separator in URLs too.
            _ => {
                let delimits = !matches!(chars.peek(), Some((_, '?' | '*' | '{')));
                flush_regex_run(&mut run, run_left_delimited, delimits, regex, on_literal);
                boundary = delimits;
            }
        }
    }

    // Without a trailing `$` (which would have flushed the run already) a matching
    // URL may continue with more token characters.
    flush_regex_run(&mut run, run_left_delimited, false, regex, on_literal);
    true
}

/// The literals a complete regex is guaranteed to contain, as strings.
#[cfg(test)]
pub(crate) fn complete_regex_literals(pattern: &str) -> Vec<String> {
    let mut literals = vec![];
    if scan_complete_regex_literals(pattern, &mut |literal: &str| {
        literals.push(literal.to_ascii_lowercase())
    }) {
        literals
    } else {
        vec![]
    }
}

/// Tokens that virtually every URL contains, so keying a filter on one of them is
/// no better than leaving it in the catch-all bucket - where it additionally gets
/// merged into a single `RegexSet` with its peers by the optimizer.
const UBIQUITOUS_TOKENS: &[&str] = &["http", "https", "www", "com", "net", "org"];

/// Extracts tokens from a complete regex filter (`/…/`) that are guaranteed to be
/// present in every URL the regex can match, see
/// [`scan_complete_regex_literals`].
///
/// Yields nothing unless at least one extracted token is selective enough to be
/// worth bucketing on, so a filter is never moved out of the catch-all bucket for a
/// token that wouldn't filter anything out anyway.
pub(crate) fn tokenize_complete_regex_to(pattern: &str, tokens_buffer: &mut TokensBuffer) {
    let initial_len = tokens_buffer.len();
    let mut selective = false;
    let complete = scan_complete_regex_literals(pattern, &mut |literal: &str| {
        // Reserve one free slot for the zero token.
        if tokens_buffer.remaining_capacity() <= 1 {
            return;
        }
        let lowercased;
        // Request URLs are tokenized after lowercasing, so tokens must be lowercase
        // even for `$match-case` filters.
        let literal = if literal.bytes().any(|b| b.is_ascii_uppercase()) {
            lowercased = literal.to_ascii_lowercase();
            lowercased.as_str()
        } else {
            literal
        };
        selective |= !UBIQUITOUS_TOKENS.contains(&literal);
        tokens_buffer.push(fast_hash(literal));
    });

    if !complete || !selective {
        tokens_buffer.truncate(initial_len);
    }
}

pub(crate) fn bin_lookup<T: Ord>(arr: &[T], elt: T) -> bool {
    arr.binary_search(&elt).is_ok()
}

#[cfg(test)]
#[path = "../tests/unit/utils.rs"]
mod unit_tests;
