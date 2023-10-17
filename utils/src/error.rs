use std::{
    error::Error,
    fmt::{Formatter, Result},
    rc::Rc,
};
use thiserror::Error;

/// Rc that implements [std::error::Error]
#[derive(Debug, Error)]
pub struct Erc {
    inner: Rc<dyn Error>,
}

impl Erc {
    pub fn new(error: impl Error + 'static) -> Self {
        let inner = Rc::new(error);
        Self { inner }
    }
}

impl std::fmt::Display for Erc {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.inner.fmt(f)
    }
}
