pub struct ArgReturnCallback<A, R> {
    cb: Box<dyn Fn(A) -> R>,
}

impl<A, R> ArgReturnCallback<A, R> {
    pub fn new(cb: Box<dyn Fn(A) -> R>) -> Self {
        Self { cb }
    }

    pub fn call(&self, arg: A) -> R {
        (self.cb)(arg)
    }
}

impl<A, R> From<&'static (dyn Fn(A) -> R)> for ArgReturnCallback<A, R> {
    fn from(cb: &'static (dyn Fn(A) -> R)) -> Self {
        Self::new(Box::new(cb))
    }
}

impl<A, R> std::fmt::Debug for ArgReturnCallback<A, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ArgReturnCallback")
    }
}

pub struct ArgCallback<A> {
    cb: Box<dyn Fn(A)>,
}

impl<A> ArgCallback<A> {
    pub fn new(cb: Box<dyn Fn(A)>) -> Self {
        Self { cb }
    }

    pub fn call(&self, arg: A) {
        (self.cb)(arg)
    }
}

impl<A> From<&'static dyn Fn(A)> for ArgCallback<A> {
    fn from(cb: &'static dyn Fn(A)) -> Self {
        Self::new(Box::new(cb))
    }
}

impl<A> std::fmt::Debug for ArgCallback<A> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ArgCallback")
    }
}

pub struct ReturnCallback<R> {
    cb: Box<dyn Fn() -> R>,
}

impl<R> ReturnCallback<R> {
    pub fn new(cb: Box<dyn Fn() -> R>) -> Self {
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

impl<R> From<&'static dyn Fn() -> R> for ReturnCallback<R> {
    fn from(cb: &'static dyn Fn() -> R) -> Self {
        Self::new(Box::new(cb))
    }
}

pub struct EmptyCallback {
    cb: Box<dyn Fn()>,
}

impl EmptyCallback {
    pub fn new(cb: Box<dyn Fn()>) -> Self {
        Self { cb }
    }

    pub fn call(&self) {
        (self.cb)()
    }
}

impl From<&'static (dyn Fn())> for EmptyCallback {
    fn from(cb: &'static (dyn Fn())) -> Self {
        Self::new(Box::new(cb))
    }
}

impl std::fmt::Debug for EmptyCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EmptyCallback")
    }
}
