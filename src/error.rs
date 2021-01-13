pub type Result<T> = std::result::Result<T, Error>;

#[derive(PartialEq, Debug)]
pub enum Error {
    InvalidSequence,
}
