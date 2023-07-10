pub struct ReturnCallback<R> {
    cb: Box<dyn Fn() -> R + Send + Sync>,
}

impl<R> ReturnCallback<R> {
    pub fn new(cb: Box<dyn Fn() -> R + Send + Sync>) -> Self {
        Self { cb }
    }

    pub fn call(&self) -> R {
        (self.cb)()
    }
}

impl<R> std::fmt::Debug for ReturnCallback<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReturnCallback")
    }
}

impl<R> From<&'static (dyn Fn() -> R + Send + Sync)> for ReturnCallback<R> {
    fn from(cb: &'static (dyn Fn() -> R + Send + Sync)) -> Self {
        Self::new(Box::new(cb))
    }
}
