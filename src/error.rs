use std::{error, fmt};

pub struct Error {
    message: String,
    source: Option<Box<dyn error::Error>>,
}

impl Error {
    pub fn new(message: &str) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unexpected error: {}", self)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.source {
            Some(err) => write!(f, "{}. Source error: {}", self.message, err),
            None => write!(f, "{}", self.message),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.source {
            Some(ref err) => Some(&**err),
            None => None,
        }
    }
}

impl From<String> for Error {
    fn from(message: String) -> Self {
        Self {
            message,
            source: None,
        }
    }
}

impl From<&str> for Error {
    fn from(message: &str) -> Self {
        Self {
            message: message.into(),
            source: None,
        }
    }
}

impl<E: error::Error + 'static> From<(String, E)> for Error {
    fn from((message, err): (String, E)) -> Self {
        Self {
            message: message,
            source: Some(Box::new(err)),
        }
    }
}

impl<E: error::Error + 'static> From<(&str, E)> for Error {
    fn from((message, err): (&str, E)) -> Self {
        Self {
            message: message.into(),
            source: Some(Box::new(err)),
        }
    }
}

impl From<Error> for String {
    fn from(err: Error) -> Self {
        format!("{}", err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
