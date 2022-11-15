pub type RawCallback<T, R> = dyn Fn(T) -> R + Send + Sync;
pub type EmptyCallback = dyn Fn() + Send + Sync;

pub type OnClickRaw = dyn Fn(u32, u32) + Send + Sync + 'static;
pub struct OnClickCallback {
    pub callback: Option<Box<OnClickRaw>>,
}

impl OnClickCallback {
    pub fn new(callback: Option<&'static OnClickRaw>) -> Self {
        Self {
            callback: callback.map(|c| Box::new(c) as Box<OnClickRaw>),
        }
    }

    pub fn call(&self, x: u32, y: u32) {
        let Some(cb) = self.callback.as_ref() else {return};
        (cb)(x, y)
    }
}

impl std::fmt::Debug for OnClickCallback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OnClickCallback")
    }
}

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
