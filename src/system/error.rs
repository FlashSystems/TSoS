use std::ffi::{IntoStringError, NulError};
use std::io;
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum Error {
	OsError(io::Error),
	ConversionError(IntoStringError),
	InvalidString(NulError),
	InvalidArgument(&'static str),
	UserNotFound(String),
	GroupNotFound(String),
	ExecFailed(i32)
}

impl fmt::Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::OsError(error) => write!(f, "Operation failed: {}", error),
			Self::ConversionError(error) => write!(f, "Wrong parameter format: {}", error),
			Self::InvalidString(error) => write!(f, "Invalid string: {}", error),
			Self::InvalidArgument(arg_name) => write!(f, "Invalid argument {}.", arg_name),
			Self::UserNotFound(user_name) => write!(f, "User {} not found.", user_name),
			Self::GroupNotFound(group_name) => write!(f, "Group {} not found.", group_name),
			Self::ExecFailed(result_code) => write!(f, "Process failed with exit code {}.", result_code)
		}
	}
}

impl error::Error for Error {
	fn source(&self) -> Option<&(dyn error::Error + 'static)> {
		match self {
			Self::OsError(error) => Some(error),
			Self::ConversionError(error) => Some(error),
			Self::InvalidString(error) => Some(error),
			_ => None
		}
	}
}

impl From<io::Error> for Error {
	fn from(error: io::Error) -> Self {
		Self::OsError(error)
	}
}

impl From<IntoStringError> for Error {
	fn from(error: IntoStringError) -> Self {
		Self::ConversionError(error)
	}
}

impl From<NulError> for Error {
	fn from(error: NulError) -> Self {
		Self::InvalidString(error)
	}
}
