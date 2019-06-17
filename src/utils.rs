#[cfg(not(target_arch = "wasm32"))]
use std::io::{BufRead, BufReader};
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
use seahash::hash;

pub type Hash = u64;
static HASH_MAX: Hash = std::u64::MAX;

#[inline]
pub fn fast_hash(input: &str) -> Hash {
    hash(input.as_bytes()) as Hash
}


fn is_allowed_filter(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '%'
}


fn is_allowed_hostname(ch: char) -> bool {
    is_allowed_filter(ch) || ch == '_' /* '_' */ || ch == '-' /* '-' */
}

pub const TOKENS_BUFFER_SIZE: usize = 128;
pub const TOKENS_BUFFER_RESERVED: usize = 1;
const TOKENS_MAX: usize = TOKENS_BUFFER_SIZE - TOKENS_BUFFER_RESERVED;

fn fast_tokenizer_no_regex(
    pattern: &str,
    is_allowed_code: &Fn(char) -> bool,
    skip_first_token: bool,
    skip_last_token: bool,
    tokens_buffer: &mut Vec<Hash>
) {
    // let mut tokens_buffer_index = 0;
    let mut inside: bool = false;
    let mut start = 0;
    let mut preceding_ch: Option<char> = None; // Used to check if a '*' is not just before a token

    for (i, c) in pattern.char_indices() {
        if tokens_buffer.len() >= TOKENS_MAX {
            return;
        }
        if is_allowed_code(c) {
            if !inside {
                inside = true;
                start = i
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
            preceding_ch = Some(c)
        } else {
            preceding_ch = Some(c)
        }
        
    }

    if !skip_last_token
        && inside
        && pattern.len() - start > 1
        && (preceding_ch != Some('*'))
    {
        let hash = fast_hash(&pattern[start..]);
        tokens_buffer.push(hash);
    }
}

fn fast_tokenizer(
    pattern: &str,
    is_allowed_code: &Fn(char) -> bool,
    skip_first_token: bool,
    skip_last_token: bool,
    tokens_buffer: &mut Vec<Hash>) {

    let mut inside: bool = false;
    let mut start = 0;
    let chars = pattern.char_indices();

    for (i, c) in chars {
        if tokens_buffer.len() >= TOKENS_MAX {
            break;
        }
        if is_allowed_code(c) {
            if !inside {
                inside = true;
                start = i;
            }
        } else if inside {
            inside = false;
            if !skip_first_token || start != 0 {
                let hash = fast_hash(&pattern[start..i]);
                tokens_buffer.push(hash);
            }
        }
    }

    if !skip_last_token && inside {
        let hash = fast_hash(&pattern[start..]);
        tokens_buffer.push(hash);
    }
}

pub fn tokenize_pooled(pattern: &str, tokens_buffer: &mut Vec<Hash>) {
    fast_tokenizer_no_regex(pattern, &is_allowed_filter, false, false, tokens_buffer);
}

pub fn tokenize(pattern: &str) -> Vec<Hash> {
    let mut tokens_buffer: Vec<Hash> = Vec::with_capacity(TOKENS_BUFFER_SIZE);
    fast_tokenizer_no_regex(pattern, &is_allowed_filter, false, false, &mut tokens_buffer);
    tokens_buffer
}


pub fn tokenize_filter(pattern: &str, skip_first_token: bool, skip_last_token: bool) -> Vec<Hash> {
    let mut tokens_buffer: Vec<Hash> = Vec::with_capacity(TOKENS_BUFFER_SIZE);
    fast_tokenizer_no_regex(pattern, &is_allowed_filter, skip_first_token, skip_last_token, &mut tokens_buffer);
    tokens_buffer
}

fn compact_tokens<T: std::cmp::Ord>(tokens: &mut Vec<T>) {
    tokens.sort_unstable();
    tokens.dedup();
}


pub fn create_fuzzy_signature(pattern: &str) -> Vec<Hash> {
    let mut tokens: Vec<Hash> = Vec::with_capacity(TOKENS_BUFFER_SIZE);
    fast_tokenizer(pattern, &is_allowed_filter, false, false, &mut tokens);
    compact_tokens(&mut tokens);
    tokens
}


pub fn create_combined_fuzzy_signature(patterns: &[String]) -> Vec<Hash> {
    let mut tokens: Vec<Hash> = Vec::with_capacity(TOKENS_BUFFER_SIZE);
    for p in patterns {
        fast_tokenizer(p, &is_allowed_filter, false, false, &mut tokens);
    }
    
    compact_tokens(&mut tokens);
    tokens
}

pub fn bin_search<T: Ord>(arr: &[T], elt: &T) -> Option<usize> {
    arr.binary_search(elt).ok()
}

pub fn bin_lookup<T: Ord>(arr: &[T], elt: T) -> bool {
    arr.binary_search(&elt).is_ok()
}

pub fn bin_lookup_optional<T: Ord>(arr: &[T], elt: Option<T>) -> bool {
    elt.map(|i| {
        arr.binary_search(&i).is_ok()
    }).unwrap_or(false)
}

pub fn has_unicode(pattern: &str) -> bool {
    let chars = pattern.chars();
    for c in chars {
        if !c.is_ascii() {
            return true
        }
    }
    false
}

const EXPECTED_RULES: usize = 75000;
#[cfg(not(target_arch = "wasm32"))]
pub fn read_file_lines(filename: &str) -> Vec<String> {
    let f = File::open(filename).unwrap_or_else(|_| panic!("File {} not found", filename));
    let reader = BufReader::new(f);
    let mut rules: Vec<String> = Vec::with_capacity(EXPECTED_RULES);
    for line in reader.lines() {
        let l = line.unwrap();
        rules.push(l);
    }
    rules.shrink_to_fit();
    rules
}
#[cfg(not(target_arch = "wasm32"))]
pub fn rules_from_lists(lists: &[String]) -> Vec<String> {
    let mut rules: Vec<String> = Vec::with_capacity(EXPECTED_RULES);
    for filename in lists {
        let mut list_rules = read_file_lines(filename);
        rules.append(&mut list_rules);
    }
    rules.shrink_to_fit();
    rules
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore] // won't match hard-coded values when using a different hash function
    fn fast_hash_matches_ts() {
        assert_eq!(fast_hash("hello world"), 4173747013); // cross-checked with the TS implementation
        assert_eq!(fast_hash("ello worl"), 2759317833); // cross-checked with the TS implementation
        assert_eq!(
            fast_hash(&"hello world"[1..10]),
            fast_hash("ello worl")
        );
        assert_eq!(fast_hash(&"hello world"[1..5]), fast_hash("ello"));
    }

    fn t(tokens: &[&str]) -> Vec<Hash> {
        tokens.into_iter().map(|t| fast_hash(&t)).collect()
    }

    #[test]
    fn tokenize_filter_works() {
        assert_eq!(
            tokenize_filter("", false, false).as_slice(),
            t(&vec![]).as_slice()
        );
        assert_eq!(
            tokenize_filter("", true, false).as_slice(),
            t(&vec![]).as_slice()
        );
        assert_eq!(
            tokenize_filter("", false, true).as_slice(),
            t(&vec![]).as_slice()
        );
        assert_eq!(
            tokenize_filter("", true, true).as_slice(),
            t(&vec![]).as_slice()
        );
        assert_eq!(
            tokenize_filter("", false, false).as_slice(),
            t(&vec![]).as_slice()
        );

        assert_eq!(
            tokenize_filter("foo/bar baz", false, false).as_slice(),
            t(&vec!["foo", "bar", "baz"]).as_slice()
        );
        assert_eq!(
            tokenize_filter("foo/bar baz", true, false).as_slice(),
            t(&vec!["bar", "baz"]).as_slice()
        );
        assert_eq!(
            tokenize_filter("foo/bar baz", true, true).as_slice(),
            t(&vec!["bar"]).as_slice()
        );
        assert_eq!(
            tokenize_filter("foo/bar baz", false, true).as_slice(),
            t(&vec!["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize_filter("foo////bar baz", false, true).as_slice(),
            t(&vec!["foo", "bar"]).as_slice()
        );
    }

    #[test]
    fn tokenize_works() {
        assert_eq!(
            tokenize("").as_slice(),
            t(&vec![]).as_slice()
        );
        assert_eq!(
            tokenize("foo").as_slice(),
            t(&vec!["foo"]).as_slice()
        );
        assert_eq!(
            tokenize("foo/bar").as_slice(),
            t(&vec!["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize("foo-bar").as_slice(),
            t(&vec!["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize("foo.bar").as_slice(),
            t(&vec!["foo", "bar"]).as_slice()
        );
        assert_eq!(
            tokenize("foo.barƬ").as_slice(),
            t(&vec!["foo", "barƬ"]).as_slice()
        );

        // Tokens cannot be surrounded by *
        assert_eq!(
            tokenize("foo.barƬ*").as_slice(),
            t(&vec!["foo"]).as_slice()
        );
        assert_eq!(
            tokenize("*foo.barƬ").as_slice(),
            t(&vec!["barƬ"]).as_slice()
        );
        assert_eq!(
            tokenize("*foo.barƬ*").as_slice(),
            t(&vec![]).as_slice()
        );
    }

    #[test]
    fn create_fuzzy_signature_works() {
        assert_eq!(create_fuzzy_signature("").as_slice(), t(&vec![]).as_slice());
        let mut tokens = t(&vec!["bar", "foo"]);
        tokens.sort_unstable();
        assert_eq!(create_fuzzy_signature("foo bar").as_slice(), tokens.as_slice());
        assert_eq!(create_fuzzy_signature("bar foo").as_slice(), tokens.as_slice());
        assert_eq!(create_fuzzy_signature("foo bar foo foo").as_slice(), tokens.as_slice());
    }

    #[test]
    fn bin_lookup_works() {
        assert_eq!(bin_lookup(&vec![], 42), false);
        assert_eq!(bin_lookup(&vec![42], 42), true);
        assert_eq!(bin_lookup(&vec![1, 2, 3, 4, 42], 42), true);
        assert_eq!(bin_lookup(&vec![1, 2, 3, 4, 42], 1), true);
        assert_eq!(bin_lookup(&vec![1, 2, 3, 4, 42], 3), true);
        assert_eq!(bin_lookup(&vec![1, 2, 3, 4, 42], 43), false);
        assert_eq!(bin_lookup(&vec![1, 2, 3, 4, 42], 0), false);
        assert_eq!(bin_lookup(&vec![1, 2, 3, 4, 42], 5), false);
    }

    #[test]
    fn bin_search_works() {
        // empty array
        assert_eq!(bin_search(&Vec::new(), &42), None);
        // array of length 1
        assert_eq!(bin_search(&vec![1], &42), None);
        assert_eq!(bin_search(&vec![42], &42), Some(0));
        // array of length 2
        assert_eq!(bin_search(&vec![0, 1], &42), None);
        assert_eq!(bin_search(&vec![1, 42], &42), Some(1));
        assert_eq!(bin_search(&vec![42, 45], &42), Some(0));
        assert_ne!(bin_search(&vec![42, 42], &42), None);

        // bigger arrays
        let data : Vec<Hash> = (1..=1000).map(|x| x*x).collect();
        assert_eq!(bin_search(&data, &42), None);
        assert_eq!(bin_search(&data, &1), Some(0));
        assert_eq!(bin_search(&data, &4), Some(1));
        assert_eq!(bin_search(&data, &(1000*1000)), Some(1000-1));
    }

    #[test]
    fn has_unicode_works() {
        let ascii: String = (b'!'..=b'~') // Start as u8
        .map(|c| c as char)
        .collect();

        assert_eq!(has_unicode(&ascii), false);
        assert_eq!(has_unicode("｡◕ ∀ ◕｡)"), true);
        assert_eq!(has_unicode("｀ｨ(´∀｀∩"), true);
        assert_eq!(has_unicode("__ﾛ(,_,*)"), true);
        assert_eq!(has_unicode("・(￣∀￣)・:*:"), true);
        assert_eq!(has_unicode("ﾟ･✿ヾ╲(｡◕‿◕｡)╱✿･ﾟ"), true);
        assert_eq!(has_unicode(",。・:*:・゜’( ☻ ω ☻ )。・:*:・゜’"), true);
        assert_eq!(has_unicode("(╯°□°）╯︵ ┻━┻)"), true);
        assert_eq!(has_unicode("(ﾉಥ益ಥ）ﾉ ┻━┻"), true);
        assert_eq!(has_unicode("┬─┬ノ( º _ ºノ)"), true);
        assert_eq!(has_unicode("( ͡° ͜ʖ ͡°)"), true);
        assert_eq!(has_unicode("¯_(ツ)_/¯"), true);
    }
}
