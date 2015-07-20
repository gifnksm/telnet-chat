use std::error::Error;
use std::fmt::{self, Display};
use std::io;
use std::sync::mpsc;

use message::{Request, Notify};

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug)]
enum AppError {
    Io(io::Error),
    MpscReqSend(mpsc::SendError<Request>),
    MpscNtfSend(mpsc::SendError<Notify>),
    MpscRecv(mpsc::RecvError)
}

impl Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AppError::Io(ref err) => write!(f, "IO error: {}", err),
            AppError::MpscReqSend(ref err) => write!(f, "MPSC Send error: {}", err),
            AppError::MpscNtfSend(ref err) => write!(f, "MPSC Send error: {}", err),
            AppError::MpscRecv(ref err) => write!(f, "MPSC Recv error: {}", err)
        }
    }
}

impl Error for AppError {
    fn description(&self) -> &str {
        match *self {
            AppError::Io(ref err) => err.description(),
            AppError::MpscReqSend(ref err) => err.description(),
            AppError::MpscNtfSend(ref err) => err.description(),
            AppError::MpscRecv(ref err) => err.description()
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            AppError::Io(ref err) => Some(err),
            AppError::MpscReqSend(ref err) => Some(err),
            AppError::MpscNtfSend(ref err) => Some(err),
            AppError::MpscRecv(ref err) => Some(err)
        }
    }
}

impl From<io::Error> for AppError {
    fn from(err: io::Error) -> AppError {
        AppError::Io(err)
    }
}
impl From<mpsc::SendError<Request>> for AppError {
    fn from(err: mpsc::SendError<Request>) -> AppError {
        AppError::MpscReqSend(err)
    }
}
impl From<mpsc::SendError<Notify>> for AppError {
    fn from(err: mpsc::SendError<Notify>) -> AppError {
        AppError::MpscNtfSend(err)
    }
}
impl From<mpsc::RecvError> for AppError {
    fn from(err: mpsc::RecvError) -> AppError {
        AppError::MpscRecv(err)
    }
}
