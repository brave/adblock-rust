//! Common utilities used by the library. Some tests and benchmarks rely on this module having
//! public visibility.

use std::ops::{Deref, DerefMut};

#[cfg(target_pointer_width = "64")]
use seahash::hash;
#[cfg(target_pointer_width = "32")]
use seahash::reference::hash;

pub type Hash = u64;

#[inline]
pub fn fast_hash(input: &str) -> Hash {
    hash(input.as_bytes()) as Hash
}

#[inline]
fn is_allowed_filter(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '%'
}

pub(crate) const TOKENS_BUFFER_SIZE: usize = 128;
pub(crate) const TOKENS_BUFFER_RESERVED: usize = 1;
const TOKENS_MAX: usize = TOKENS_BUFFER_SIZE - TOKENS_BUFFER_RESERVED;

#[derive(Clone, Debug)]
pub struct StackVec<T, const N: usize> {
    data: [T; N],
    len: usize,
}

impl<T, const N: usize> StackVec<T, N> {
    pub fn new() -> Self
    where
        T: Default + Copy,
    {
        Self {
            data: [Default::default(); N],
            len: 0,
        }
    }

    pub fn push(&mut self, value: T) -> Result<(), &'static str>
    where
        T: Copy,
    {
        if self.len < N {
            self.data[self.len] = value;
            self.len += 1;
            Ok(())
        } else {
            Err("Too many tokens")
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[inline]
    pub fn get(&self, index: usize) -> Option<&T> {
        if index < self.len {
            Some(&self.data[index])
        } else {
            None
        }
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.data[..self.len].iter()
    }

    #[inline]
    #[allow(dead_code)]
    pub fn as_slice(&self) -> &[T] {
        &self.data[..self.len]
    }
}

impl<'a, T, const N: usize> IntoIterator for &'a StackVec<T, N> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T, const N: usize> Deref for StackVec<T, N> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data[..self.len]
    }
}

impl<T, const N: usize> DerefMut for StackVec<T, N> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data[..self.len]
    }
}

pub type Tokens = StackVec<Hash, TOKENS_BUFFER_SIZE>;

fn fast_tokenizer_no_regex(
    pattern: &str,
    is_allowed_code: &dyn Fn(char) -> bool,
    skip_first_token: bool,
    skip_last_token: bool,
) -> Tokens {
    // let mut tokens_buffer_index = 0;
    let mut inside: bool = false;
    let mut start = 0;
    let mut preceding_ch: Option<char> = None; // Used to check if a '*' is not just before a token
    let mut tokens = Tokens::new();

    for (i, c) in pattern.char_indices() {
        if tokens.len() >= TOKENS_MAX {
            return tokens;
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
                if !tokens.push(hash).is_ok() {
                    break;
                }
            }
            preceding_ch = Some(c);
        } else {
            preceding_ch = Some(c);
        }
    }

    if !skip_last_token && inside && pattern.len() - start > 1 && (preceding_ch != Some('*')) {
        let hash = fast_hash(&pattern[start..]);
        let _ = tokens.push(hash);
    }
    tokens
}

pub fn tokenize(pattern: &str) -> Tokens {
    fast_tokenizer_no_regex(pattern, &is_allowed_filter, false, false)
}

pub(crate) fn tokenize_filter(
    pattern: &str,
    skip_first_token: bool,
    skip_last_token: bool,
) -> Tokens {
    fast_tokenizer_no_regex(
        pattern,
        &is_allowed_filter,
        skip_first_token,
        skip_last_token,
    )
}

pub(crate) fn bin_lookup<T: Ord>(arr: &[T], elt: T) -> bool {
    arr.binary_search(&elt).is_ok()
}

#[cfg(test)]
#[path = "../tests/unit/utils.rs"]
mod unit_tests;
