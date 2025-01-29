pub mod prelude {
    pub use crate::{assert_return, ok, ok_err, some, some_err};
}

/// It will be Some or else it will return ().
/// Allows multiple idents separated by commas.
#[macro_export]
macro_rules! some {
    ($($some:ident),+) => {
        $(
            let Some($some) = $some else {
                return;
            };
        )+
    };
    ($some:expr) => {
        match $some {
            Some(some) => some,
            None => return,
        }
    };
}

/// It will be Some or else it will return () with an error.
/// Allows multiple idents separated by commas.
#[macro_export]
macro_rules! some_err {
    ($($some:ident),+) => {
        $(
            let Some($some) = $some else {
                error!("Expected Some({}), found None.", std::stringify!($some));
                return;
            };
        )+
    };
}

/// It will be Ok or else it will return ().
/// Allows multiple idents separated by commas.
#[macro_export]
macro_rules! ok {
    ($($ok:ident),+) => {
        $(
            let Ok($ok) = $ok else {
                return;
            };
        )+
    };
}

/// It will be Ok or else it will return () with an error.
/// Allows multiple idents separated by commas.
#[macro_export]
macro_rules! ok_err {
    ($($ok:ident),+) => {
        $(
            let $ok = match $ok {
                Ok(ok) => ok,
                Err(err) => {
                    error!("Expected Some({}), found Err({}).", std::stringify!($some), err);
                    return;
                }
            };
        )+
    };

    ($ok:expr) => {
        match $ok {
            Ok(ok) => ok,
            Err(err) => {
                error!("Expected Some({}), found Err({}).", std::stringify!($some), err);
                return;
            }
        }
    };
}

/// It will be true or else it will return ().
/// Allows multiple idents separated by commas.
#[macro_export]
macro_rules! assert_return {
    ($($bool:expr),+) => {
        $(
            if !($bool) {
                return;
            }
        )+
    };
}
