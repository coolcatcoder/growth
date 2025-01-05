use crate::prelude::*;

pub mod prelude {
    pub use super::{AppFunction, RegistrationPlugin};
}

/// Iterates through the collected app functions and runs each of them.
pub struct RegistrationPlugin;

impl Plugin for RegistrationPlugin {
    fn build(&self, app: &mut App) {
        inventory::iter::<AppFunction>
            .into_iter()
            .for_each(|app_function| {
                (app_function.0)(app);
            });
    }
}

/// A function that gets run on an app.
/// While you can use inventory::collect with this struct, you can and should instead use our app macro.
pub struct AppFunction(pub fn(&mut App));

collect!(AppFunction);

/// Accepts a closure or a function's ident. Expects an input of &mut app, with no output.
/// Example:
/// ```
/// app!(|app| {
///     app.add_systems(Startup, || info!("Fun!"));
/// });
/// ```
#[macro_export]
macro_rules! app {
    ($function: expr) => {
        submit! {
            AppFunction($function)
        }
    };
}

/// Give it a resource type and it will init it.
#[macro_export]
macro_rules! init_resource {
    ($resource: ty) => {
        submit! {
            AppFunction(|app| {
                app.init_resource::<$resource>();
            })
        }
    };
}
