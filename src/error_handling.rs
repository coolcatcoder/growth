pub mod prelude {
    pub use crate::some;
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
}
