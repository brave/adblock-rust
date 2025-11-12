//! Common utilities used by the library. Some tests and benchmarks rely on this module having
//! public visibility.

#[cfg(target_pointer_width = "64")]
use seahash::hash;
#[cfg(target_pointer_width = "32")]
use seahash::reference::hash;

/// A stack-allocated vector that uses [T; MAX_SIZE] with Default initialization.
/// All elements are initialized to T::default(), and we track the logical size separately.
/// Note: a future impl can switch to using MaybeUninit with unsafe code for better efficiency.
pub struct ArrayVec<T, const MAX_SIZE: usize> {
    data: [T; MAX_SIZE],
    size: usize,
}

impl<T: Default + Copy, const MAX_SIZE: usize> Default for ArrayVec<T, MAX_SIZE> {
    fn default() -> Self {
        Self {
            data: [T::default(); MAX_SIZE],
            size: 0,
        }
    }
}

impl<T: Default, const MAX_SIZE: usize> ArrayVec<T, MAX_SIZE> {
    pub fn push(&mut self, value: T) -> bool {
        if self.size < MAX_SIZE {
            self.data[self.size] = value;
            self.size += 1;
            true
        } else {
            false
        }
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0
    }

    pub fn get_free_capacity(&self) -> usize {
        MAX_SIZE - self.size
    }

    pub fn clear(&mut self) {
        self.size = 0;
    }

    pub fn as_slice(&self) -> &[T] {
        &self.data[..self.size]
    }

    pub fn into_vec(mut self) -> Vec<T> {
        let mut v = Vec::with_capacity(self.size);
        for i in 0..self.size {
            v.push(std::mem::take(&mut self.data[i]));
        }
        self.size = 0;
        v
    }
}

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

pub type TokensBuffer = ArrayVec<Hash, 256>;

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
        if tokens_buffer.get_free_capacity() <= 1 {
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
    tokens_buffer.into_vec()
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
    tokens_buffer.into_vec()
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

pub(crate) fn bin_lookup<T: Ord>(arr: &[T], elt: T) -> bool {
    arr.binary_search(&elt).is_ok()
}

#[cfg(test)]
#[path = "../tests/unit/utils.rs"]
mod unit_tests;
