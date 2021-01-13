use std::fmt::{Debug, Formatter};
use std::iter::Peekable;
use std::str::Chars;

// -------------------------------------------------------------------------------------------------

type Result<T> = std::result::Result<T, Error>;

#[derive(PartialEq, Debug)]
pub enum Error {
    EOF,
    Unexpected,
}

impl From<Error> for crate::error::Error {
    fn from(_: Error) -> Self {
        crate::error::Error::InvalidSequence
    }
}

// -------------------------------------------------------------------------------------------------

/// A simple allocation-free string lexer.
#[derive(Debug, Clone)]
pub struct Lexer<'a> {
    cursor: &'a str,
    cursor_saved: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(string: &'a str) -> Self {
        Lexer {
            cursor: string,
            cursor_saved: string,
        }
    }

    /// Extracts all characters that match a pattern.
    ///
    /// ## Arguments
    ///
    /// - `pattern`: The pattern predicate.
    ///
    /// ## Returns
    ///
    /// A `&str` slice containing matching characters, or `None` if there's nothing left.
    /// 
    /// ## State
    /// 
    /// The lexer cursor will advance by however many characters were extracted.
    pub fn extract(&mut self, pattern: impl Fn(char) -> bool) -> Result<&'a str> {
        //
        // PERFORMANCE: Although this implementation looks weirdly inefficient, it's safer and
        //              faster than using `char_indices()` and get_unchecked().
        //
        if self.cursor.is_empty() {
            return Err(Error::EOF);
        }

        let mut iter = self.cursor.chars();
        let mut last_iter = iter.clone();

        // Advance the iterator until we reach either the end, or a character past the pattern.
        while let Some(c) = iter.next() {
            if !pattern(c) {
                break;
            }

            last_iter.clone_from(&iter);
        }

        // Using the position of the last acceptable character, we can create a &str that contains
        // all of the characters that weren't extracted by the predicate.
        let remaining = last_iter.as_str();

        // Using the length of the original cursor size and the remaining characters, we can then
        // create a &str that contains all of the extracted characters.
        let extracted = &self.cursor[0..(self.cursor.len() - remaining.len())];

        // And finally, we update the cursor and return the extracted characters.
        self.cursor = remaining;
        Ok(extracted)
    }

    /// Extracts one character that matches a pattern.
    ///
    /// ## Arguments
    ///
    /// - `pattern`: The pattern predicate.
    ///
    /// ## Returns
    ///
    /// A `&str` slice containing the matching character.
    /// If the character does not match, it returns [Error::Unexpected] instead.
    /// 
    /// ## State
    /// 
    /// The lexer cursor will advance if a character was extracted.
    pub fn extract_one(&mut self, pattern: impl Fn(char) -> bool) -> Result<&'a str> {
        let mut iter = self.cursor.char_indices();

        if let Some((_, c)) = iter.next() {
            return if pattern(c) {
                let remaining = iter.as_str();
                let extracted = match iter.next() {
                    None => self.cursor,
                    Some((i, _)) => &self.cursor[0..i],
                };

                self.cursor = remaining;
                Ok(extracted)
            } else {
                Err(Error::Unexpected)
            };
        }

        Err(Error::EOF)
    }

    /// Extracts one character that matches a pattern.
    /// This is the greedy variant that will always advance the cursor.
    ///
    /// ## Arguments
    ///
    /// - `pattern`: The pattern predicate.
    ///
    /// ## Returns
    ///
    /// A `&str` slice containing the matching character.
    /// If the character does not match, it returns [Error::Unexpected] instead.
    /// 
    /// ## State
    /// 
    /// The lexer cursor will advance by one character.
    #[inline]
    pub fn extract_one_greedy(&mut self, pattern: impl Fn(char) -> bool) -> Result<&'a str> {
        match self.extract_one(pattern) {
            Err(Error::Unexpected) => {
                self.skip(1)?;
                Err(Error::Unexpected)
            },
            other => other
        }
    }

    /// Marks the current cursor position.
    #[inline(always)]
    pub fn mark(&mut self) {
        self.cursor_saved = self.cursor;
    }

    /// Rewinds the cursor back to the marked position.
    #[inline(always)]
    pub fn rewind(&mut self) {
        self.cursor = self.cursor_saved;
    }

    /// Gets a string of the characters consumed since the marked position.
    pub fn consumed(&self) -> &'a str {
        // SAFETY: 1. `cursor_saved` is a substring of the same source string as `cursor`.
        //         2. `cursor` is always either at the same position or ahead of `cursor_saved`. 
        //         3. We're creating the slice from a str, so it is safe to turn it back into a str.
        let start = self.cursor_saved.as_ptr();
        let length = unsafe { self.cursor.as_ptr().sub(start as usize) as usize };
        unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(start, length)) }
    }

    /// Gets the remaining characters that haven't been extracted.
    #[inline(always)]
    pub fn remaining(&self) -> &'a str {
        self.cursor
    }

    /// Returns `true` if there are no more characters left to be extracted.
    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.cursor.is_empty()
    }
    
    /// Skips a number of characters.
    /// 
    /// ## Arguments
    ///
    /// - `n`: The number of characters to skip.
    ///
    /// ## Returns
    ///
    /// If this would skip past the end of the input, this returns [Error::Unexpected].
    /// 
    /// ## State
    /// 
    /// If there are `n` characters available to skip, the lexer cursor will advance by `n` characters.
    /// Otherwise, no state changes will occur.
    fn skip(&mut self, mut n: usize) -> Result<()> {
        let mut iter = self.cursor.chars();
        
        while n > 0 {
            n -= 1;
            if iter.next() == None {
                return Err(Error::EOF);
            }
        }
        
        self.cursor = iter.as_str();
        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::lex::*;

    #[test]
    fn extract() {
        let mut lex = Lexer::new("hello123 world");
        assert_eq!(lex.is_empty(), false);

        // Extract "hello", and ensure the lexer has " 123 world" remaining.
        assert_eq!(lex.extract(char::is_alphabetic), Ok("hello"));
        assert_eq!(lex.remaining(), "123 world");
        assert_eq!(lex.is_empty(), false);

        // Extract one character, but it doesn't match.
        assert_eq!(lex.extract_one(char::is_alphabetic), Err(Error::Unexpected));
        assert_eq!(lex.remaining(), "123 world");

        // Extract one character greedily, but it doesn't match.
        assert_eq!(lex.extract_one_greedy(char::is_alphabetic), Err(Error::Unexpected));
        assert_eq!(lex.remaining(), "23 world");

        // Extract one character.
        assert_eq!(lex.extract_one(char::is_numeric), Ok("2"));
        assert_eq!(lex.remaining(), "3 world");

        // Extract anything not alphabetic, and ensure the lexer has "world" remaining.
        assert_eq!(lex.extract(|c| !char::is_alphabetic(c)), Ok("3 "));
        assert_eq!(lex.remaining(), "world");

        // Extract, but nothing matches.
        assert_eq!(lex.extract(|_| false), Ok(""));
        assert_eq!(lex.remaining(), "world");

        // Extract the rest of it.
        assert_eq!(lex.extract(|_| true), Ok("world"));
        assert_eq!(lex.remaining(), "");
        assert_eq!(lex.is_empty(), true);

        // Ensure it still works with empty contents.
        assert_eq!(lex.extract(|_| true), Err(Error::EOF));
        assert_eq!(lex.extract_one(|_| true), Err(Error::EOF));
        assert_eq!(lex.remaining(), "");
        assert_eq!(lex.is_empty(), true);
    }

    #[test]
    fn mark() {
        let mut lex = Lexer::new("hello123 world");

        // Extract "hello", and ensure the consumed characters are also "hello".
        assert_eq!(lex.extract(char::is_alphabetic), Ok("hello"));
        assert_eq!(lex.consumed(), "hello");

        // Extract "123", and ensure the consumed characters are "hello123".
        assert_eq!(lex.extract(char::is_numeric), Ok("123"));
        assert_eq!(lex.consumed(), "hello123");
        
        // Rewind to the last-marked position (implicitly, the beginning).
        lex.rewind();
        assert_eq!(lex.remaining(), "hello123 world");

        // Extract "hello" and mark.
        assert_eq!(lex.extract(char::is_alphabetic), Ok("hello"));
        lex.mark();
        assert_eq!(lex.consumed(), "");
        assert_eq!(lex.remaining(), "123 world");
    }

    #[test]
    fn skip() {
        let mut lex = Lexer::new("12345");
        assert_eq!(lex.remaining(), "12345");

        // Skip 0.
        assert_eq!(lex.skip(0), Ok(()));
        assert_eq!(lex.remaining(), "12345");

        // Skip 1.
        assert_eq!(lex.skip(1), Ok(()));
        assert_eq!(lex.remaining(), "2345");

        // Skip 2.
        assert_eq!(lex.skip(2), Ok(()));
        assert_eq!(lex.remaining(), "45");

        // Skip past the end.
        assert_eq!(lex.skip(10), Err(Error::EOF));
        assert_eq!(lex.remaining(), "45");

        // Skip while nothing remaining.
        let mut lex = Lexer::new("");
        assert_eq!(lex.skip(1), Err(Error::EOF));
        assert_eq!(lex.remaining(), "");
    }

    // TODO: extract() test, with unicode.
}
