use crate::error::Error;
use crate::error::Result;
use crate::lex::Lexer;
use std::str::FromStr;
use std::sync::atomic::Ordering::SeqCst;

/// An ANSI escape sequence.
#[derive(Eq, PartialEq, Debug)]
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
#[derive(Eq, PartialEq, Debug)]
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
#[derive(Eq, PartialEq, Debug)]
pub struct ControlSequence<'a> {
    parameters: &'a str,
    intermediates: &'a str,
    finalizer: &'a str,
}

/// A variable-length string, as defined in the ANSI standard.
///
/// This is implicitly created by a preceding sequence.
/// The string is terminated by `ESC '\'`, or `BEL`.
#[derive(Eq, PartialEq, Debug)]
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

impl<'a> Parse<'a> for ControlSequence<'a> {
    fn parse(lexer: &mut Lexer<'a>) -> Result<Self> {
        lexer.extract_one(is_sequence_opener)?;

        match lexer.extract_one_greedy(is_csi_finalizer)? {
            "[" => Ok(ControlSequence {
                parameters: lexer.extract(is_csi_parameter)?,
                intermediates: lexer.extract(is_csi_intermediate)?,
                finalizer: lexer.extract_one_greedy(is_csi_finalizer)?,
            }),
            _ => Err(Error::InvalidSequence),
        }
        
    }
}

impl<'a> Parse<'a> for AnsiString<'a> {
    fn parse(lexer: &mut Lexer<'a>) -> Result<Self> {
        let text = lexer.extract(|c| !is_st_opener(c))?;
        let finalizer = match lexer.extract_one_greedy(is_st_opener)? {
            "\x07" => "\x07",
            "\x1B" => if lexer.extract_one_greedy(|c| c == '\\')? == "\\" {
                "\x1B\\"
            } else {
                return Err(Error::InvalidSequence);
            },
            _ => return Err(Error::InvalidSequence),
        };

        Ok(AnsiString {
            text,
            finalizer,
        })
    }
}

impl<'a> Parse<'a> for Sequence<'a> {
    fn parse(lexer: &mut Lexer<'a>) -> Result<Self> {
        let mut lookahead_lexer = lexer.clone();
        let lookahead = AnsiSequence::parse(&mut lookahead_lexer)?;
        
        if lookahead.intermediates.is_empty() {
            return Ok(match lookahead.finalizer {
                "[" => Sequence::CSI(ControlSequence::parse(lexer)?), 
                "]" => Sequence::OSC(AnsiSequence::parse(lexer)?, AnsiString::parse(lexer)?),
                _ => Sequence::Regular(AnsiSequence::parse(lexer)?)
            })
        }
        
        Ok(Sequence::Regular(AnsiSequence::parse(lexer)?))
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

/// Checks if a character is an opener for the ST sequence.
/// 
/// On xterm, this is either `ESC \`, or BEL. 
pub(crate) fn is_st_opener(c: char) -> bool {
    c == '\x07' || is_sequence_opener(c)
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
    fn parse_csi_sequence() {
        let mut lex = Lexer::new("\x1B[33m\x1B[48;5;105m");

        // Parse valid `ESC [ 33 m` sequence.
        assert_eq!(
            ControlSequence::parse(&mut lex),
            Ok(ControlSequence {
                parameters: "33",
                intermediates: "",
                finalizer: "m",
            })
        );

        // Parse valid `ESC [ 48;5;105 m` sequence.
        assert_eq!(
            ControlSequence::parse(&mut lex),
            Ok(ControlSequence {
                parameters: "48;5;105",
                intermediates: "",
                finalizer: "m",
            })
        );

        // Ensure nothing is left to read.
        assert!(lex.is_empty());
    }

    #[test]
    fn parse_string_sequence() {
        let mut lex = Lexer::new("Test\x1B\\Strings\x07\x1B[33m");

        // Parse valid `... ESC \\` sequence.
        assert_eq!(
            AnsiString::parse(&mut lex),
            Ok(AnsiString {
                text: "Test",
                finalizer: "\x1B\\",
            })
        );

        // Parse valid `... BEL` sequence.
        assert_eq!(
            AnsiString::parse(&mut lex),
            Ok(AnsiString {
                text: "Strings",
                finalizer: "\x07",
            })
        );
        
        // Parse invalid string sequence.
        assert_eq!(
            AnsiString::parse(&mut lex),
            Err(Error::InvalidSequence)
        );
    }

    #[test]
    fn parse_invalid_sequence() {
        let mut lex = Lexer::new("\x1B\x1B");
        assert_eq!(AnsiSequence::parse(&mut lex), Err(Error::InvalidSequence));
        assert_eq!(lex.remaining(), "");
    }


    #[test]
    fn parse_sequences() {
        let mut lex = Lexer::new("\x1B7\x1B[38;2;10;25;255m\x1B]0;Title\x07");

        // Parse regular sequence.
        assert_eq!(
            Sequence::parse(&mut lex),
            Ok(Sequence::Regular(AnsiSequence {
                intermediates: "",
                finalizer: "7",
            }))
        );

        // Parse CSI sequence.
        assert_eq!(
            Sequence::parse(&mut lex),
            Ok(Sequence::CSI(ControlSequence {
                parameters: "38;2;10;25;255",
                intermediates: "",
                finalizer: "m",
            }))
        );

        // Parse OSC sequence.
        assert_eq!(
            Sequence::parse(&mut lex),
            Ok(Sequence::OSC(AnsiSequence {
                intermediates: "",
                finalizer: "]",
            }, AnsiString {
                text: "0;Title",
                finalizer: "\x07"
            }))
        );

        // Ensure nothing is left to read.
        assert!(lex.is_empty());
    }
    
}
