/// Result Alias with BackendError
pub type Result<T> = std::result::Result<T, BackendError>;

/// Error type of PyroscopeBackend
#[non_exhaustive]
#[derive(thiserror::Error, Debug)]
pub enum BackendError {
    #[error("Other: {}", &.0)]
    AdHoc(String),

    #[error("{msg}: {source:?}")]
    Compat {
        msg: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync + 'static>,
    },

    #[error(transparent)]
    Pprof(#[from] pprof::Error),

    #[error(transparent)]
    Io(#[from] std::io::Error),

    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

impl BackendError {
    /// Create a new instance of PyroscopeError
    pub fn new(msg: &str) -> Self {
        BackendError::AdHoc(msg.to_string())
    }

    /// Create a new instance of PyroscopeError with source
    pub fn new_with_source<E>(msg: &str, source: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        BackendError::Compat {
            msg: msg.to_string(),
            source: Box::new(source),
        }
    }
}
