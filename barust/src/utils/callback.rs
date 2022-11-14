pub type RawCallback<T, R> = dyn Fn(T) -> R + Send + Sync;
pub type EmptyCallback = dyn Fn() + Send + Sync;

pub struct Callback<T, R> {
    callback: Box<RawCallback<T, R>>,
}

impl<T, R> Callback<T, R> {
    pub fn new(callback: Box<RawCallback<T, R>>) -> Self {
        Self { callback }
    }

    pub fn call(&self, arg: T) -> R {
        (self.callback)(arg)
    }
}

impl<T, R> From<&'static RawCallback<T, R>> for Callback<T, R> {
    fn from(c: &'static RawCallback<T, R>) -> Self {
        Self {
            callback: Box::new(c),
        }
    }
}

impl From<&'static EmptyCallback> for Callback<(), ()> {
    fn from(c: &'static EmptyCallback) -> Self {
        Self {
            callback: Box::new(|()| c()),
        }
    }
}

impl<T, R> std::fmt::Debug for Callback<T, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Callback")
    }
}
