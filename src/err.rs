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


impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::IO(e)
    }
}
impl From<DecodingError> for Error {
    fn from(e: DecodingError) -> Error {
        Error::Decoding(e)
    }
}
impl From<String> for Error {
    fn from(e: String) -> Error {
        Error::Other(e)
    }
}
impl From<&'static str> for Error {
    fn from(e: &'static str) -> Error {
        Error::Other(e.to_string())
    }
}


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
