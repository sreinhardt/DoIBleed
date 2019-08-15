use std::error::Error;
use std::fmt;
use std::io::ErrorKind;
use std::io::Error as IoError;

pub type DoIBleedResult<T> = Result<T, DoIBleedError>;

#[derive(Debug)]
pub enum DoIBleedError {
  StdIo(IoError),
  Bitlab(String),
  MsgTooLarge(usize),
}

impl fmt::Display for DoIBleedError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      DoIBleedError::MsgTooLarge(ref s) =>
        write!{f, "DoIBleed: Message was too large = {}", s},
      DoIBleedError::Bitlab(ref s) => write!{f, "DoIBleed: {}", s},
      ref e @ _ => write!{f, "DoIBleed: {}", e.description()},
    }
  }
}
impl Error for DoIBleedError {
  fn description(&self) -> &str {
    match *self {
      DoIBleedError::MsgTooLarge(_) => "Message was too large",
      DoIBleedError::StdIo(ref e) => e.description(),
      DoIBleedError::Bitlab(ref e) => e,
    }
  }
}
impl Into<IoError> for DoIBleedError {
  fn into(self) -> IoError {
    IoError::new(ErrorKind::Other, self)
  }
}
impl From<IoError> for DoIBleedError {
  fn from(err: IoError) -> DoIBleedError {
    DoIBleedError::StdIo(err)
  }
}
impl From<String> for DoIBleedError {
  fn from(err: String) -> DoIBleedError {
    DoIBleedError::Bitlab(err)
  }
}
