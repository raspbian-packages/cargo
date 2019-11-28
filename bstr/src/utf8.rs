use core::char;
use core::cmp;
use core::fmt;
#[cfg(feature = "std")]
use std::error;

use ascii;

// The UTF-8 decoder provided here is based on the one presented here:
// https://bjoern.hoehrmann.de/utf-8/decoder/dfa/
//
// We *could* have done UTF-8 decoding by using a DFA generated by `\p{any}`
// using regex-automata that is roughly the same size. The real benefit of
// Hoehrmann's formulation is that the byte class mapping below is manually
// tailored such that each byte's class doubles as a shift to mask out the
// bits necessary for constructing the leading bits of each codepoint value
// from the initial byte.
//
// There are some minor differences between this implementation and Hoehrmann's
// formulation.
//
// Firstly, we make REJECT have state ID 0, since it makes the state table
// itself a little easier to read and is consistent with the notion that 0
// means "false" or "bad."
//
// Secondly, when doing bulk decoding, we add a SIMD accelerated ASCII fast
// path.
//
// Thirdly, we pre-multiply the state IDs to avoid a multiplication instruction
// in the core decoding loop. (Which is what regex-automata would do by
// default.)
//
// Fourthly, we split the byte class mapping and transition table into two
// arrays because it's clearer.
//
// It is unlikely that this is the fastest way to do UTF-8 decoding, however,
// it is fairly simple.

const ACCEPT: usize = 12;
const REJECT: usize = 0;

#[cfg_attr(rustfmt, rustfmt::skip)]
static CLASSES: [u8; 256] = [
   0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
   0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
   0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
   0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,  0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
   1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,  9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,9,
   7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,  7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,7,
   8,8,2,2,2,2,2,2,2,2,2,2,2,2,2,2,  2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
  10,3,3,3,3,3,3,3,3,3,3,3,3,4,3,3, 11,6,6,6,5,8,8,8,8,8,8,8,8,8,8,8,
];

#[cfg_attr(rustfmt, rustfmt::skip)]
static STATES_FORWARD: &'static [u8] = &[
  0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
  12, 0, 24, 36, 60, 96, 84, 0, 0, 0, 48, 72,
  0, 12, 0, 0, 0, 0, 0, 12, 0, 12, 0, 0,
  0, 24, 0, 0, 0, 0, 0, 24, 0, 24, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 24, 0, 0, 0, 0,
  0, 24, 0, 0, 0, 0, 0, 0, 0, 24, 0, 0,
  0, 0, 0, 0, 0, 0, 0, 36, 0, 36, 0, 0,
  0, 36, 0, 0, 0, 0, 0, 36, 0, 36, 0, 0,
  0, 36, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
];

/// An iterator over Unicode scalar values in a byte string.
///
/// When invalid UTF-8 byte sequences are found, they are substituted with the
/// Unicode replacement codepoint (`U+FFFD`) using the
/// ["maximal subpart" strategy](http://www.unicode.org/review/pr-121.html).
///
/// This iterator is created by the
/// [`chars`](trait.ByteSlice.html#method.chars) method provided by the
/// [`ByteSlice`](trait.ByteSlice.html) extension trait for `&[u8]`.
#[derive(Clone, Debug)]
pub struct Chars<'a> {
    bs: &'a [u8],
}

impl<'a> Chars<'a> {
    pub(crate) fn new(bs: &'a [u8]) -> Chars<'a> {
        Chars { bs }
    }

    /// View the underlying data as a subslice of the original data.
    ///
    /// The slice returned has the same lifetime as the original slice, and so
    /// the iterator can continue to be used while this exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use bstr::ByteSlice;
    ///
    /// let mut chars = b"abc".chars();
    ///
    /// assert_eq!(b"abc", chars.as_bytes());
    /// chars.next();
    /// assert_eq!(b"bc", chars.as_bytes());
    /// chars.next();
    /// chars.next();
    /// assert_eq!(b"", chars.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.bs
    }
}

impl<'a> Iterator for Chars<'a> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<char> {
        let (ch, size) = decode_lossy(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[size..];
        Some(ch)
    }
}

impl<'a> DoubleEndedIterator for Chars<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<char> {
        let (ch, size) = decode_last_lossy(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[..self.bs.len() - size];
        Some(ch)
    }
}

/// An iterator over Unicode scalar values in a byte string and their
/// byte index positions.
///
/// When invalid UTF-8 byte sequences are found, they are substituted with the
/// Unicode replacement codepoint (`U+FFFD`) using the
/// ["maximal subpart" strategy](http://www.unicode.org/review/pr-121.html).
///
/// Note that this is slightly different from the `CharIndices` iterator
/// provided by the standard library. Aside from working on possibly invalid
/// UTF-8, this iterator provides both the corresponding starting and ending
/// byte indices of each codepoint yielded. The ending position is necessary to
/// slice the original byte string when invalid UTF-8 bytes are converted into
/// a Unicode replacement codepoint, since a single replacement codepoint can
/// substitute anywhere from 1 to 3 invalid bytes (inclusive).
///
/// This iterator is created by the
/// [`char_indices`](trait.ByteSlice.html#method.char_indices) method provided
/// by the [`ByteSlice`](trait.ByteSlice.html) extension trait for `&[u8]`.
#[derive(Clone, Debug)]
pub struct CharIndices<'a> {
    bs: &'a [u8],
    forward_index: usize,
    reverse_index: usize,
}

impl<'a> CharIndices<'a> {
    pub(crate) fn new(bs: &'a [u8]) -> CharIndices<'a> {
        CharIndices { bs: bs, forward_index: 0, reverse_index: bs.len() }
    }

    /// View the underlying data as a subslice of the original data.
    ///
    /// The slice returned has the same lifetime as the original slice, and so
    /// the iterator can continue to be used while this exists.
    ///
    /// # Examples
    ///
    /// ```
    /// use bstr::ByteSlice;
    ///
    /// let mut it = b"abc".char_indices();
    ///
    /// assert_eq!(b"abc", it.as_bytes());
    /// it.next();
    /// assert_eq!(b"bc", it.as_bytes());
    /// it.next();
    /// it.next();
    /// assert_eq!(b"", it.as_bytes());
    /// ```
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        self.bs
    }
}

impl<'a> Iterator for CharIndices<'a> {
    type Item = (usize, usize, char);

    #[inline]
    fn next(&mut self) -> Option<(usize, usize, char)> {
        let index = self.forward_index;
        let (ch, size) = decode_lossy(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[size..];
        self.forward_index += size;
        Some((index, index + size, ch))
    }
}

impl<'a> DoubleEndedIterator for CharIndices<'a> {
    #[inline]
    fn next_back(&mut self) -> Option<(usize, usize, char)> {
        let (ch, size) = decode_last_lossy(self.bs);
        if size == 0 {
            return None;
        }
        self.bs = &self.bs[..self.bs.len() - size];
        self.reverse_index -= size;
        Some((self.reverse_index, self.reverse_index + size, ch))
    }
}

/// An error that occurs when UTF-8 decoding fails.
///
/// This error occurs when attempting to convert a non-UTF-8 byte
/// string to a Rust string that must be valid UTF-8. For example,
/// [`to_str`](trait.ByteSlice.html#method.to_str) is one such method.
///
/// # Example
///
/// This example shows what happens when a given byte sequence is invalid,
/// but ends with a sequence that is a possible prefix of valid UTF-8.
///
/// ```
/// use bstr::{B, ByteSlice};
///
/// let s = B(b"foobar\xF1\x80\x80");
/// let err = s.to_str().unwrap_err();
/// assert_eq!(err.valid_up_to(), 6);
/// assert_eq!(err.error_len(), None);
/// ```
///
/// This example shows what happens when a given byte sequence contains
/// invalid UTF-8.
///
/// ```
/// use bstr::ByteSlice;
///
/// let s = b"foobar\xF1\x80\x80quux";
/// let err = s.to_str().unwrap_err();
/// assert_eq!(err.valid_up_to(), 6);
/// // The error length reports the maximum number of bytes that correspond to
/// // a valid prefix of a UTF-8 encoded codepoint.
/// assert_eq!(err.error_len(), Some(3));
///
/// // In contrast to the above which contains a single invalid prefix,
/// // consider the case of multiple individal bytes that are never valid
/// // prefixes. Note how the value of error_len changes!
/// let s = b"foobar\xFF\xFFquux";
/// let err = s.to_str().unwrap_err();
/// assert_eq!(err.valid_up_to(), 6);
/// assert_eq!(err.error_len(), Some(1));
///
/// // The fact that it's an invalid prefix does not change error_len even
/// // when it immediately precedes the end of the string.
/// let s = b"foobar\xFF";
/// let err = s.to_str().unwrap_err();
/// assert_eq!(err.valid_up_to(), 6);
/// assert_eq!(err.error_len(), Some(1));
/// ```
#[derive(Debug, Eq, PartialEq)]
pub struct Utf8Error {
    valid_up_to: usize,
    error_len: Option<usize>,
}

impl Utf8Error {
    /// Returns the byte index of the position immediately following the last
    /// valid UTF-8 byte.
    ///
    /// # Example
    ///
    /// This examples shows how `valid_up_to` can be used to retrieve a
    /// possibly empty prefix that is guaranteed to be valid UTF-8:
    ///
    /// ```
    /// use bstr::ByteSlice;
    ///
    /// let s = b"foobar\xF1\x80\x80quux";
    /// let err = s.to_str().unwrap_err();
    ///
    /// // This is guaranteed to never panic.
    /// let string = s[..err.valid_up_to()].to_str().unwrap();
    /// assert_eq!(string, "foobar");
    /// ```
    #[inline]
    pub fn valid_up_to(&self) -> usize {
        self.valid_up_to
    }

    /// Returns the total number of invalid UTF-8 bytes immediately following
    /// the position returned by `valid_up_to`. This value is always at least
    /// `1`, but can be up to `3` if bytes form a valid prefix of some UTF-8
    /// encoded codepoint.
    ///
    /// If the end of the original input was found before a valid UTF-8 encoded
    /// codepoint could be completed, then this returns `None`. This is useful
    /// when processing streams, where a `None` value signals that more input
    /// might be needed.
    #[inline]
    pub fn error_len(&self) -> Option<usize> {
        self.error_len
    }
}

#[cfg(feature = "std")]
impl error::Error for Utf8Error {
    fn description(&self) -> &str {
        "invalid UTF-8"
    }
}

impl fmt::Display for Utf8Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid UTF-8 found at byte offset {}", self.valid_up_to)
    }
}

/// Returns OK if and only if the given slice is completely valid UTF-8.
///
/// If the slice isn't valid UTF-8, then an error is returned that explains
/// the first location at which invalid UTF-89 was detected.
pub fn validate(slice: &[u8]) -> Result<(), Utf8Error> {
    // The fast path for validating UTF-8. It steps through a UTF-8 automaton
    // and uses a SIMD accelerated ASCII fast path on x86_64. If an error is
    // detected, it backs up and runs the slower version of the UTF-8 automaton
    // to determine correct error information.
    fn fast(slice: &[u8]) -> Result<(), Utf8Error> {
        let mut state = ACCEPT;
        let mut i = 0;

        while i < slice.len() {
            let b = slice[i];

            // ASCII fast path. If we see two consecutive ASCII bytes, then try
            // to validate as much ASCII as possible very quickly.
            if state == ACCEPT
                && b <= 0x7F
                && slice.get(i + 1).map_or(false, |&b| b <= 0x7F)
            {
                i += ascii::first_non_ascii_byte(&slice[i..]);
                continue;
            }

            state = step(state, b);
            if state == REJECT {
                return Err(find_valid_up_to(slice, i));
            }
            i += 1;
        }
        if state != ACCEPT {
            Err(find_valid_up_to(slice, slice.len()))
        } else {
            Ok(())
        }
    }

    // Given the first position at which a UTF-8 sequence was determined to be
    // invalid, return an error that correctly reports the position at which
    // the last complete UTF-8 sequence ends.
    #[inline(never)]
    fn find_valid_up_to(slice: &[u8], rejected_at: usize) -> Utf8Error {
        // In order to find the last valid byte, we need to back up an amount
        // that guarantees every preceding byte is part of a valid UTF-8
        // code unit sequence. To do this, we simply locate the last leading
        // byte that occurs before rejected_at.
        let mut backup = rejected_at.saturating_sub(1);
        while backup > 0 && !is_leading_utf8_byte(slice[backup]) {
            backup -= 1;
        }
        let upto = cmp::min(slice.len(), rejected_at.saturating_add(1));
        let mut err = slow(&slice[backup..upto]).unwrap_err();
        err.valid_up_to += backup;
        err
    }

    // Like top-level UTF-8 decoding, except it correctly reports a UTF-8 error
    // when an invalid sequence is found. This is split out from validate so
    // that the fast path doesn't need to keep track of the position of the
    // last valid UTF-8 byte. In particular, tracking this requires checking
    // for an ACCEPT state on each byte, which degrades throughput pretty
    // badly.
    fn slow(slice: &[u8]) -> Result<(), Utf8Error> {
        let mut state = ACCEPT;
        let mut valid_up_to = 0;
        for (i, &b) in slice.iter().enumerate() {
            state = step(state, b);
            if state == ACCEPT {
                valid_up_to = i + 1;
            } else if state == REJECT {
                // Our error length must always be at least 1.
                let error_len = Some(cmp::max(1, i - valid_up_to));
                return Err(Utf8Error { valid_up_to, error_len });
            }
        }
        if state != ACCEPT {
            Err(Utf8Error { valid_up_to, error_len: None })
        } else {
            Ok(())
        }
    }

    // Advance to the next state given the current state and current byte.
    fn step(state: usize, b: u8) -> usize {
        let class = CLASSES[b as usize];
        // SAFETY: This is safe because 'class' is always <=11 and 'state' is
        // always <=96. Therefore, the maximal index is 96+11 = 107, where
        // STATES_FORWARD.len() = 108 such that every index is guaranteed to be
        // valid by construction of the state machine and the byte equivalence
        // classes.
        unsafe {
            *STATES_FORWARD.get_unchecked(state + class as usize) as usize
        }
    }

    fast(slice)
}

/// UTF-8 decode a single Unicode scalar value from the beginning of a slice.
///
/// When successful, the corresponding Unicode scalar value is returned along
/// with the number of bytes it was encoded with. The number of bytes consumed
/// for a successful decode is always between 1 and 4, inclusive.
///
/// When unsuccessful, `None` is returned along with the number of bytes that
/// make up a maximal prefix of a valid UTF-8 code unit sequence. In this case,
/// the number of bytes consumed is always between 0 and 3, inclusive, where
/// 0 is only returned when `slice` is empty.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use bstr::decode_utf8;
///
/// // Decoding a valid codepoint.
/// let (ch, size) = decode_utf8(b"\xE2\x98\x83");
/// assert_eq!(Some('☃'), ch);
/// assert_eq!(3, size);
///
/// // Decoding an incomplete codepoint.
/// let (ch, size) = decode_utf8(b"\xE2\x98");
/// assert_eq!(None, ch);
/// assert_eq!(2, size);
/// ```
///
/// This example shows how to iterate over all codepoints in UTF-8 encoded
/// bytes, while replacing invalid UTF-8 sequences with the replacement
/// codepoint:
///
/// ```
/// use bstr::{B, decode_utf8};
///
/// let mut bytes = B(b"\xE2\x98\x83\xFF\xF0\x9D\x9E\x83\xE2\x98\x61");
/// let mut chars = vec![];
/// while !bytes.is_empty() {
///     let (ch, size) = decode_utf8(bytes);
///     bytes = &bytes[size..];
///     chars.push(ch.unwrap_or('\u{FFFD}'));
/// }
/// assert_eq!(vec!['☃', '\u{FFFD}', '𝞃', '\u{FFFD}', 'a'], chars);
/// ```
#[inline]
pub fn decode<B: AsRef<[u8]>>(slice: B) -> (Option<char>, usize) {
    let slice = slice.as_ref();
    match slice.get(0) {
        None => return (None, 0),
        Some(&b) if b <= 0x7F => return (Some(b as char), 1),
        _ => {}
    }

    let (mut state, mut cp, mut i) = (ACCEPT, 0, 0);
    while i < slice.len() {
        decode_step(&mut state, &mut cp, slice[i]);
        i += 1;

        if state == ACCEPT {
            // SAFETY: This is safe because `decode_step` guarantees that
            // `cp` is a valid Unicode scalar value in an ACCEPT state.
            let ch = unsafe { char::from_u32_unchecked(cp) };
            return (Some(ch), i);
        } else if state == REJECT {
            // At this point, we always want to advance at least one byte.
            return (None, cmp::max(1, i.saturating_sub(1)));
        }
    }
    (None, i)
}

/// Lossily UTF-8 decode a single Unicode scalar value from the beginning of a
/// slice.
///
/// When successful, the corresponding Unicode scalar value is returned along
/// with the number of bytes it was encoded with. The number of bytes consumed
/// for a successful decode is always between 1 and 4, inclusive.
///
/// When unsuccessful, the Unicode replacement codepoint (`U+FFFD`) is returned
/// along with the number of bytes that make up a maximal prefix of a valid
/// UTF-8 code unit sequence. In this case, the number of bytes consumed is
/// always between 0 and 3, inclusive, where 0 is only returned when `slice` is
/// empty.
///
/// # Examples
///
/// Basic usage:
///
/// ```ignore
/// use bstr::decode_utf8_lossy;
///
/// // Decoding a valid codepoint.
/// let (ch, size) = decode_utf8_lossy(b"\xE2\x98\x83");
/// assert_eq!('☃', ch);
/// assert_eq!(3, size);
///
/// // Decoding an incomplete codepoint.
/// let (ch, size) = decode_utf8_lossy(b"\xE2\x98");
/// assert_eq!('\u{FFFD}', ch);
/// assert_eq!(2, size);
/// ```
///
/// This example shows how to iterate over all codepoints in UTF-8 encoded
/// bytes, while replacing invalid UTF-8 sequences with the replacement
/// codepoint:
///
/// ```ignore
/// use bstr::{B, decode_utf8_lossy};
///
/// let mut bytes = B(b"\xE2\x98\x83\xFF\xF0\x9D\x9E\x83\xE2\x98\x61");
/// let mut chars = vec![];
/// while !bytes.is_empty() {
///     let (ch, size) = decode_utf8_lossy(bytes);
///     bytes = &bytes[size..];
///     chars.push(ch);
/// }
/// assert_eq!(vec!['☃', '\u{FFFD}', '𝞃', '\u{FFFD}', 'a'], chars);
/// ```
#[inline]
pub fn decode_lossy<B: AsRef<[u8]>>(slice: B) -> (char, usize) {
    match decode(slice) {
        (Some(ch), size) => (ch, size),
        (None, size) => ('\u{FFFD}', size),
    }
}

/// UTF-8 decode a single Unicode scalar value from the end of a slice.
///
/// When successful, the corresponding Unicode scalar value is returned along
/// with the number of bytes it was encoded with. The number of bytes consumed
/// for a successful decode is always between 1 and 4, inclusive.
///
/// When unsuccessful, `None` is returned along with the number of bytes that
/// make up a maximal prefix of a valid UTF-8 code unit sequence. In this case,
/// the number of bytes consumed is always between 0 and 3, inclusive, where
/// 0 is only returned when `slice` is empty.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use bstr::decode_last_utf8;
///
/// // Decoding a valid codepoint.
/// let (ch, size) = decode_last_utf8(b"\xE2\x98\x83");
/// assert_eq!(Some('☃'), ch);
/// assert_eq!(3, size);
///
/// // Decoding an incomplete codepoint.
/// let (ch, size) = decode_last_utf8(b"\xE2\x98");
/// assert_eq!(None, ch);
/// assert_eq!(2, size);
/// ```
///
/// This example shows how to iterate over all codepoints in UTF-8 encoded
/// bytes in reverse, while replacing invalid UTF-8 sequences with the
/// replacement codepoint:
///
/// ```
/// use bstr::{B, decode_last_utf8};
///
/// let mut bytes = B(b"\xE2\x98\x83\xFF\xF0\x9D\x9E\x83\xE2\x98\x61");
/// let mut chars = vec![];
/// while !bytes.is_empty() {
///     let (ch, size) = decode_last_utf8(bytes);
///     bytes = &bytes[..bytes.len()-size];
///     chars.push(ch.unwrap_or('\u{FFFD}'));
/// }
/// assert_eq!(vec!['a', '\u{FFFD}', '𝞃', '\u{FFFD}', '☃'], chars);
/// ```
#[inline]
pub fn decode_last<B: AsRef<[u8]>>(slice: B) -> (Option<char>, usize) {
    // TODO: We could implement this by reversing the UTF-8 automaton, but for
    // now, we do it the slow way by using the forward automaton.

    let slice = slice.as_ref();
    if slice.is_empty() {
        return (None, 0);
    }
    let mut start = slice.len() - 1;
    let limit = slice.len().saturating_sub(4);
    while start > limit && !is_leading_utf8_byte(slice[start]) {
        start -= 1;
    }
    let (ch, size) = decode(&slice[start..]);
    // If we didn't consume all of the bytes, then that means there's at least
    // one stray byte that never occurs in a valid code unit prefix, so we can
    // advance by one byte.
    if start + size != slice.len() {
        (None, 1)
    } else {
        (ch, size)
    }
}

/// Lossily UTF-8 decode a single Unicode scalar value from the end of a slice.
///
/// When successful, the corresponding Unicode scalar value is returned along
/// with the number of bytes it was encoded with. The number of bytes consumed
/// for a successful decode is always between 1 and 4, inclusive.
///
/// When unsuccessful, the Unicode replacement codepoint (`U+FFFD`) is returned
/// along with the number of bytes that make up a maximal prefix of a valid
/// UTF-8 code unit sequence. In this case, the number of bytes consumed is
/// always between 0 and 3, inclusive, where 0 is only returned when `slice` is
/// empty.
///
/// # Examples
///
/// Basic usage:
///
/// ```ignore
/// use bstr::decode_last_utf8_lossy;
///
/// // Decoding a valid codepoint.
/// let (ch, size) = decode_last_utf8_lossy(b"\xE2\x98\x83");
/// assert_eq!('☃', ch);
/// assert_eq!(3, size);
///
/// // Decoding an incomplete codepoint.
/// let (ch, size) = decode_last_utf8_lossy(b"\xE2\x98");
/// assert_eq!('\u{FFFD}', ch);
/// assert_eq!(2, size);
/// ```
///
/// This example shows how to iterate over all codepoints in UTF-8 encoded
/// bytes in reverse, while replacing invalid UTF-8 sequences with the
/// replacement codepoint:
///
/// ```ignore
/// use bstr::decode_last_utf8_lossy;
///
/// let mut bytes = B(b"\xE2\x98\x83\xFF\xF0\x9D\x9E\x83\xE2\x98\x61");
/// let mut chars = vec![];
/// while !bytes.is_empty() {
///     let (ch, size) = decode_last_utf8_lossy(bytes);
///     bytes = &bytes[..bytes.len()-size];
///     chars.push(ch);
/// }
/// assert_eq!(vec!['a', '\u{FFFD}', '𝞃', '\u{FFFD}', '☃'], chars);
/// ```
#[inline]
pub fn decode_last_lossy<B: AsRef<[u8]>>(slice: B) -> (char, usize) {
    match decode_last(slice) {
        (Some(ch), size) => (ch, size),
        (None, size) => ('\u{FFFD}', size),
    }
}

#[inline]
pub fn decode_step(state: &mut usize, cp: &mut u32, b: u8) {
    let class = CLASSES[b as usize];
    if *state == ACCEPT {
        *cp = (0xFF >> class) & (b as u32);
    } else {
        *cp = (b as u32 & 0b111111) | (*cp << 6);
    }
    *state = STATES_FORWARD[*state + class as usize] as usize;
}

fn is_leading_utf8_byte(b: u8) -> bool {
    // In the ASCII case, the most significant bit is never set. The leading
    // byte of a 2/3/4-byte sequence always has the top two most significant
    // bigs set.
    (b & 0b1100_0000) != 0b1000_0000
}

#[cfg(test)]
mod tests {
    use std::char;

    use ext_slice::{ByteSlice, B};
    use tests::LOSSY_TESTS;
    use utf8::{self, Utf8Error};

    fn utf8e(valid_up_to: usize) -> Utf8Error {
        Utf8Error { valid_up_to, error_len: None }
    }

    fn utf8e2(valid_up_to: usize, error_len: usize) -> Utf8Error {
        Utf8Error { valid_up_to, error_len: Some(error_len) }
    }

    #[test]
    fn validate_all_codepoints() {
        for i in 0..(0x10FFFF + 1) {
            let cp = match char::from_u32(i) {
                None => continue,
                Some(cp) => cp,
            };
            let mut buf = [0; 4];
            let s = cp.encode_utf8(&mut buf);
            assert_eq!(Ok(()), utf8::validate(s.as_bytes()));
        }
    }

    #[test]
    fn validate_multiple_codepoints() {
        assert_eq!(Ok(()), utf8::validate(b"abc"));
        assert_eq!(Ok(()), utf8::validate(b"a\xE2\x98\x83a"));
        assert_eq!(Ok(()), utf8::validate(b"a\xF0\x9D\x9C\xB7a"));
        assert_eq!(Ok(()), utf8::validate(b"\xE2\x98\x83\xF0\x9D\x9C\xB7",));
        assert_eq!(
            Ok(()),
            utf8::validate(b"a\xE2\x98\x83a\xF0\x9D\x9C\xB7a",)
        );
        assert_eq!(
            Ok(()),
            utf8::validate(b"\xEF\xBF\xBD\xE2\x98\x83\xEF\xBF\xBD",)
        );
    }

    #[test]
    fn validate_errors() {
        // single invalid byte
        assert_eq!(Err(utf8e2(0, 1)), utf8::validate(b"\xFF"));
        // single invalid byte after ASCII
        assert_eq!(Err(utf8e2(1, 1)), utf8::validate(b"a\xFF"));
        // single invalid byte after 2 byte sequence
        assert_eq!(Err(utf8e2(2, 1)), utf8::validate(b"\xCE\xB2\xFF"));
        // single invalid byte after 3 byte sequence
        assert_eq!(Err(utf8e2(3, 1)), utf8::validate(b"\xE2\x98\x83\xFF"));
        // single invalid byte after 4 byte sequence
        assert_eq!(Err(utf8e2(4, 1)), utf8::validate(b"\xF0\x9D\x9D\xB1\xFF"));

        // An invalid 2-byte sequence with a valid 1-byte prefix.
        assert_eq!(Err(utf8e2(0, 1)), utf8::validate(b"\xCE\xF0"));
        // An invalid 3-byte sequence with a valid 2-byte prefix.
        assert_eq!(Err(utf8e2(0, 2)), utf8::validate(b"\xE2\x98\xF0"));
        // An invalid 4-byte sequence with a valid 3-byte prefix.
        assert_eq!(Err(utf8e2(0, 3)), utf8::validate(b"\xF0\x9D\x9D\xF0"));

        // An overlong sequence. Should be \xE2\x82\xAC, but we encode the
        // same codepoint value in 4 bytes. This not only tests that we reject
        // overlong sequences, but that we get valid_up_to correct.
        assert_eq!(Err(utf8e2(0, 1)), utf8::validate(b"\xF0\x82\x82\xAC"));
        assert_eq!(Err(utf8e2(1, 1)), utf8::validate(b"a\xF0\x82\x82\xAC"));
        assert_eq!(
            Err(utf8e2(3, 1)),
            utf8::validate(b"\xE2\x98\x83\xF0\x82\x82\xAC",)
        );

        // Check that encoding a surrogate codepoint using the UTF-8 scheme
        // fails validation.
        assert_eq!(Err(utf8e2(0, 1)), utf8::validate(b"\xED\xA0\x80"));
        assert_eq!(Err(utf8e2(1, 1)), utf8::validate(b"a\xED\xA0\x80"));
        assert_eq!(
            Err(utf8e2(3, 1)),
            utf8::validate(b"\xE2\x98\x83\xED\xA0\x80",)
        );

        // Check that an incomplete 2-byte sequence fails.
        assert_eq!(Err(utf8e2(0, 1)), utf8::validate(b"\xCEa"));
        assert_eq!(Err(utf8e2(1, 1)), utf8::validate(b"a\xCEa"));
        assert_eq!(
            Err(utf8e2(3, 1)),
            utf8::validate(b"\xE2\x98\x83\xCE\xE2\x98\x83",)
        );
        // Check that an incomplete 3-byte sequence fails.
        assert_eq!(Err(utf8e2(0, 2)), utf8::validate(b"\xE2\x98a"));
        assert_eq!(Err(utf8e2(1, 2)), utf8::validate(b"a\xE2\x98a"));
        assert_eq!(
            Err(utf8e2(3, 2)),
            utf8::validate(b"\xE2\x98\x83\xE2\x98\xE2\x98\x83",)
        );
        // Check that an incomplete 4-byte sequence fails.
        assert_eq!(Err(utf8e2(0, 3)), utf8::validate(b"\xF0\x9D\x9Ca"));
        assert_eq!(Err(utf8e2(1, 3)), utf8::validate(b"a\xF0\x9D\x9Ca"));
        assert_eq!(
            Err(utf8e2(4, 3)),
            utf8::validate(b"\xF0\x9D\x9C\xB1\xF0\x9D\x9C\xE2\x98\x83",)
        );
        assert_eq!(
            Err(utf8e2(6, 3)),
            utf8::validate(b"foobar\xF1\x80\x80quux",)
        );

        // Check that an incomplete (EOF) 2-byte sequence fails.
        assert_eq!(Err(utf8e(0)), utf8::validate(b"\xCE"));
        assert_eq!(Err(utf8e(1)), utf8::validate(b"a\xCE"));
        assert_eq!(Err(utf8e(3)), utf8::validate(b"\xE2\x98\x83\xCE"));
        // Check that an incomplete (EOF) 3-byte sequence fails.
        assert_eq!(Err(utf8e(0)), utf8::validate(b"\xE2\x98"));
        assert_eq!(Err(utf8e(1)), utf8::validate(b"a\xE2\x98"));
        assert_eq!(Err(utf8e(3)), utf8::validate(b"\xE2\x98\x83\xE2\x98"));
        // Check that an incomplete (EOF) 4-byte sequence fails.
        assert_eq!(Err(utf8e(0)), utf8::validate(b"\xF0\x9D\x9C"));
        assert_eq!(Err(utf8e(1)), utf8::validate(b"a\xF0\x9D\x9C"));
        assert_eq!(
            Err(utf8e(4)),
            utf8::validate(b"\xF0\x9D\x9C\xB1\xF0\x9D\x9C",)
        );

        // Test that we errors correct even after long valid sequences. This
        // checks that our "backup" logic for detecting errors is correct.
        assert_eq!(
            Err(utf8e2(8, 1)),
            utf8::validate(b"\xe2\x98\x83\xce\xb2\xe3\x83\x84\xFF",)
        );
    }

    #[test]
    fn decode_valid() {
        fn d(mut s: &str) -> Vec<char> {
            let mut chars = vec![];
            while !s.is_empty() {
                let (ch, size) = utf8::decode(s.as_bytes());
                s = &s[size..];
                chars.push(ch.unwrap());
            }
            chars
        }

        assert_eq!(vec!['☃'], d("☃"));
        assert_eq!(vec!['☃', '☃'], d("☃☃"));
        assert_eq!(vec!['α', 'β', 'γ', 'δ', 'ε'], d("αβγδε"));
        assert_eq!(vec!['☃', '⛄', '⛇'], d("☃⛄⛇"));
        assert_eq!(
            vec!['𝗮', '𝗯', '𝗰', '𝗱', '𝗲'],
            d("𝗮𝗯𝗰𝗱𝗲")
        );
    }

    #[test]
    fn decode_invalid() {
        let (ch, size) = utf8::decode(b"");
        assert_eq!(None, ch);
        assert_eq!(0, size);

        let (ch, size) = utf8::decode(b"\xFF");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode(b"\xCE\xF0");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode(b"\xE2\x98\xF0");
        assert_eq!(None, ch);
        assert_eq!(2, size);

        let (ch, size) = utf8::decode(b"\xF0\x9D\x9D");
        assert_eq!(None, ch);
        assert_eq!(3, size);

        let (ch, size) = utf8::decode(b"\xF0\x9D\x9D\xF0");
        assert_eq!(None, ch);
        assert_eq!(3, size);

        let (ch, size) = utf8::decode(b"\xF0\x82\x82\xAC");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode(b"\xED\xA0\x80");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode(b"\xCEa");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode(b"\xE2\x98a");
        assert_eq!(None, ch);
        assert_eq!(2, size);

        let (ch, size) = utf8::decode(b"\xF0\x9D\x9Ca");
        assert_eq!(None, ch);
        assert_eq!(3, size);
    }

    #[test]
    fn decode_lossy() {
        let (ch, size) = utf8::decode_lossy(b"");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(0, size);

        let (ch, size) = utf8::decode_lossy(b"\xFF");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_lossy(b"\xCE\xF0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_lossy(b"\xE2\x98\xF0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(2, size);

        let (ch, size) = utf8::decode_lossy(b"\xF0\x9D\x9D\xF0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(3, size);

        let (ch, size) = utf8::decode_lossy(b"\xF0\x82\x82\xAC");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_lossy(b"\xED\xA0\x80");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_lossy(b"\xCEa");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_lossy(b"\xE2\x98a");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(2, size);

        let (ch, size) = utf8::decode_lossy(b"\xF0\x9D\x9Ca");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(3, size);
    }

    #[test]
    fn decode_last_valid() {
        fn d(mut s: &str) -> Vec<char> {
            let mut chars = vec![];
            while !s.is_empty() {
                let (ch, size) = utf8::decode_last(s.as_bytes());
                s = &s[..s.len() - size];
                chars.push(ch.unwrap());
            }
            chars
        }

        assert_eq!(vec!['☃'], d("☃"));
        assert_eq!(vec!['☃', '☃'], d("☃☃"));
        assert_eq!(vec!['ε', 'δ', 'γ', 'β', 'α'], d("αβγδε"));
        assert_eq!(vec!['⛇', '⛄', '☃'], d("☃⛄⛇"));
        assert_eq!(
            vec!['𝗲', '𝗱', '𝗰', '𝗯', '𝗮'],
            d("𝗮𝗯𝗰𝗱𝗲")
        );
    }

    #[test]
    fn decode_last_invalid() {
        let (ch, size) = utf8::decode_last(b"");
        assert_eq!(None, ch);
        assert_eq!(0, size);

        let (ch, size) = utf8::decode_last(b"\xFF");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last(b"\xCE\xF0");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last(b"\xCE");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last(b"\xE2\x98\xF0");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last(b"\xE2\x98");
        assert_eq!(None, ch);
        assert_eq!(2, size);

        let (ch, size) = utf8::decode_last(b"\xF0\x9D\x9D\xF0");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last(b"\xF0\x9D\x9D");
        assert_eq!(None, ch);
        assert_eq!(3, size);

        let (ch, size) = utf8::decode_last(b"\xF0\x82\x82\xAC");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last(b"\xED\xA0\x80");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last(b"\xED\xA0");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last(b"\xED");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last(b"a\xCE");
        assert_eq!(None, ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last(b"a\xE2\x98");
        assert_eq!(None, ch);
        assert_eq!(2, size);

        let (ch, size) = utf8::decode_last(b"a\xF0\x9D\x9C");
        assert_eq!(None, ch);
        assert_eq!(3, size);
    }

    #[test]
    fn decode_last_lossy() {
        let (ch, size) = utf8::decode_last_lossy(b"");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(0, size);

        let (ch, size) = utf8::decode_last_lossy(b"\xFF");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last_lossy(b"\xCE\xF0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last_lossy(b"\xCE");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last_lossy(b"\xE2\x98\xF0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last_lossy(b"\xE2\x98");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(2, size);

        let (ch, size) = utf8::decode_last_lossy(b"\xF0\x9D\x9D\xF0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last_lossy(b"\xF0\x9D\x9D");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(3, size);

        let (ch, size) = utf8::decode_last_lossy(b"\xF0\x82\x82\xAC");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last_lossy(b"\xED\xA0\x80");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last_lossy(b"\xED\xA0");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last_lossy(b"\xED");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last_lossy(b"a\xCE");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(1, size);

        let (ch, size) = utf8::decode_last_lossy(b"a\xE2\x98");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(2, size);

        let (ch, size) = utf8::decode_last_lossy(b"a\xF0\x9D\x9C");
        assert_eq!('\u{FFFD}', ch);
        assert_eq!(3, size);
    }

    #[test]
    fn chars() {
        for (i, &(expected, input)) in LOSSY_TESTS.iter().enumerate() {
            let got: String = B(input).chars().collect();
            assert_eq!(
                expected, got,
                "chars(ith: {:?}, given: {:?})",
                i, input,
            );
            let got: String =
                B(input).char_indices().map(|(_, _, ch)| ch).collect();
            assert_eq!(
                expected, got,
                "char_indices(ith: {:?}, given: {:?})",
                i, input,
            );

            let expected: String = expected.chars().rev().collect();

            let got: String = B(input).chars().rev().collect();
            assert_eq!(
                expected, got,
                "chars.rev(ith: {:?}, given: {:?})",
                i, input,
            );
            let got: String =
                B(input).char_indices().rev().map(|(_, _, ch)| ch).collect();
            assert_eq!(
                expected, got,
                "char_indices.rev(ith: {:?}, given: {:?})",
                i, input,
            );
        }
    }
}
