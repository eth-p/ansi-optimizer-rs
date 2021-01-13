#![allow(clippy::match_like_matches_macro)]
#![cfg_attr(test, allow(unused_variables, unused_imports, dead_code))]

// Modules.
mod ansi;
pub mod error;
pub mod lex; // TODO: Remove pub.

// Exports.
pub use error::Error;

// Imports.
use std::fmt::{Display, Formatter};

// -------------------------------------------------------------------------------------------------

/// An ANSI escape sequence optimizer.
///
/// This will consume a series of ANSI/VT100 escape sequences and generate equivalent sequences.
/// The sequences will be comprised of a smaller or equal number of characters than the input.
///
/// To create the optimized sequence, the [ToString] trait or [Display] trait should be used:
///
/// ```
/// # use ansi_optimizer::Optimizer;
/// let mut optimizer = Optimizer::new();
/// optimizer.update("\x1B[33;41m");
/// optimizer.update("\x1B[39m");
///
/// assert_eq!(optimizer.to_string(), "\x1B[41m");
/// ```
#[derive(Clone, Debug, Default)]
pub struct Optimizer {
    // TODO: Internal representation.
}

impl Optimizer {
    /// Creates a new optimizer with a default state.
    pub fn new() -> Self {
        Optimizer {}
    }

    /// Resets the optimizer back to a default state.
    /// This is equivalent to creating a new optimizer, but avoids unnecessary allocations.
    #[inline]
    pub fn reset(&mut self) {
        unimplemented!()
    }

    /// Updates
    #[inline]
    pub fn update(&mut self, sequence: impl AsRef<str>) -> Result<(), Error> {
        unimplemented!()
    }
}

impl Display for Optimizer {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        unimplemented!()
    }
}

// extern crate peekmore;
//
// mod ansi;
// mod state;
//
// #[cfg(test)]
// mod tests {
//     #[test]
//     fn it_works() {
//         assert_eq!(2 + 2, 4);
//     }
// }
