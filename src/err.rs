use std;
use std::result;
use bincode::rustc_serialize::DecodingError;

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    Decoding(DecodingError),
    Other(String),

    // Networking
    WrongProtocol,
    UnknownMessage,
}

macro_rules! implement_froms_for_error {
  ($($i:ident : $t:ty => $e:expr),*,) => { implement_froms_for_error![$($i: $t => $e),*]; };
  ($($i:ident : $t:ty => $e:expr),*) => {
    $(impl From<$t> for Error {
        fn from($i: $t) -> Error {
          $e
        }
      }
    )*
  };
}

implement_froms_for_error![
  e: std::io::Error => Error::IO(e),
  e: DecodingError => Error::Decoding(e),
  e: String => Error::Other(e),
  e: &'static str => Error::Other(e.to_string()),
];

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let _ = std::fmt::Debug::fmt(&self, f)?;
        Ok(())
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self  {
            Error::Other(ref desc) => &desc,
            Error::Decoding(ref err) => err.description(),
            Error::IO(ref err) => err.description(),

            Error::WrongProtocol => "Wrong protocol number.",
            Error::UnknownMessage => "Unknown message number.",
        }
    }
}
