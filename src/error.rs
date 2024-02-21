pub type Result<T> = std::result::Result<T, Error>;

// The Error type bellow is used in development only. In production, you should use a enum Error.
pub type Error = Box<dyn std::error::Error>;
