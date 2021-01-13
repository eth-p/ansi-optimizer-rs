use crate::error::Error;
use crate::error::Result;
use crate::lex::Lexer;
use std::str::FromStr;

/// An ANSI escape sequence.
#[derive(Debug)]
pub enum Sequence<'a> {
    CSI(ControlSequence<'a>),
    OSC(AnsiSequence<'a>, AnsiString<'a>),
    Regular(AnsiSequence<'a>),
}

/// A regular ANSI escape sequence:
///
/// ```text
/// ESC I* F
/// ```
#[derive(PartialEq, Debug)]
pub struct AnsiSequence<'a> {
    intermediates: &'a str,
    finalizer: &'a str,
}

/// An ANSI control sequence.
///
/// ```text
/// ESC '['
/// ```
///
/// CSI format:
///
/// ```text
/// P* I* F
/// ```
#[derive(Debug)]
pub struct ControlSequence<'a> {
    parameters: &'a str,
    intermediates: &'a str,
    finalizer: &'a str,
}

/// A variable-length string, as defined in the ANSI standard.
///
/// This is implicitly created by a preceding sequence.
/// The string is terminated by `ESC '\'`, or `BEL`.
#[derive(Debug)]
pub struct AnsiString<'a> {
    text: &'a str,
    finalizer: &'a str,
}

// -------------------------------------------------------------------------------------------------

trait Parse<'a> {
    fn parse(lexer: &mut Lexer<'a>) -> Result<Self>
    where
        Self: Sized;
}

impl<'a> Parse<'a> for AnsiSequence<'a> {
    fn parse(lexer: &mut Lexer<'a>) -> Result<Self> {
        lexer.extract_one(is_sequence_opener)?;

        Ok(AnsiSequence {
            intermediates: lexer.extract(is_sequence_intermediate)?,
            finalizer: lexer.extract_one_greedy(is_sequence_finalizer)?,
        })
    }
}

// -------------------------------------------------------------------------------------------------

/// Checks if a character is an ANSI sequence opener.
///
/// The opener is the ESC control character, and denotes the beginning of an escape sequence.
/// It is followed by zero or more intermediate bytes, which are followed by a finalizer.
pub(crate) fn is_sequence_opener(c: char) -> bool {
    c == '\x1B'
}

/// Checks if a character is an ANSI sequence finalizer byte.
///
/// The finalizer is an ASCII character between 0x30 and 0x7E inclusive, and it
/// denotes the end of an ANSI escape sequence.
pub(crate) fn is_sequence_finalizer(c: char) -> bool {
    match c {
        '\x30'..='\x7E' => true,
        _ => false,
    }
}

/// Checks if a character is an ANSI sequence intermediate byte.
///
/// Intermediate bytes are ASCII characters between 0x20 and 0x2F inclusive.
/// Zero or more may be located between the escape character and the finalizer.
pub(crate) fn is_sequence_intermediate(c: char) -> bool {
    match c {
        '\x20'..='\x2F' => true,
        _ => false,
    }
}

/// Checks if a character is an ANSI control sequence finalizer byte.
///
/// The control sequence finalizer is an ASCII character between 0x40 and 0x7E inclusive.
/// It is used similarly to the regular ANSI finalizer, but specifically for CSI commands.
pub(crate) fn is_csi_finalizer(c: char) -> bool {
    match c {
        '\x40'..='\x7E' => true,
        _ => false,
    }
}

/// Checks if a character is an ANSI control sequence parameter byte.
///
/// The control sequence parameter is an ASCII character between 0x30 and 0x3F inclusive.
/// It is used similarly to the regular ANSI finalizer, but specifically for CSI commands.
pub(crate) fn is_csi_parameter(c: char) -> bool {
    match c {
        '\x30'..='\x3F' => true,
        _ => false,
    }
}

/// Checks if a character is an ANSI control sequence intermediate byte.
///
/// Intermediate bytes are ASCII characters between 0x20 and 0x2F inclusive.
pub(crate) fn is_csi_intermediate(c: char) -> bool {
    match c {
        '\x20'..='\x2F' => true,
        _ => false,
    }
}

// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::ansi::*;

    #[test]
    fn parse_basic_sequence() {
        let mut lex = Lexer::new("\x1BX\x1B$!c");

        // Parse valid `ESC F` sequence.
        assert_eq!(
            AnsiSequence::parse(&mut lex),
            Ok(AnsiSequence {
                intermediates: "",
                finalizer: "X",
            })
        );

        // Parse valid `ESC I F` sequence.
        assert_eq!(
            AnsiSequence::parse(&mut lex),
            Ok(AnsiSequence {
                intermediates: "$!",
                finalizer: "c",
            })
        );
        
        // Ensure nothing is left to read.
        assert!(lex.is_empty());
    }

    #[test]
    fn parse_invalid_sequence() {
        let mut lex = Lexer::new("\x1B\x1B");
        assert_eq!(AnsiSequence::parse(&mut lex), Err(Error::InvalidSequence));
        assert_eq!(lex.remaining(), "");
    }

    // TODO: extract() test, with unicode.
}
