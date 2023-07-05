use std::rc::Rc;
use thiserror::Error;

/// Rc that implements [std::error::Error]
#[derive(Debug, Error)]
pub struct Erc {
    inner: Rc<dyn std::error::Error>,
}

impl Erc {
    pub fn new<E: std::error::Error + 'static>(error: E) -> Self {
        let inner = Rc::new(error);
        Self { inner }
    }
}

impl std::fmt::Display for Erc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}
