pub type Result<T> = core::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    AnyError(String),
}

/// Allow string slices to be converted to Error
impl From<&str> for Error {
    fn from(val: &str) -> Self {
        Self::AnyError(val.to_string())
    }
}

/// Allow strings to be converted to Error
impl From<String> for Error {
    fn from(val: String) -> Self {
        Self::AnyError(val)
    }
}

/// Allow errors to be displayed as string
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::AnyError(val) => write!(f, "{}", val),
        }
    }
}
