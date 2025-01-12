use crate::prelude::*;

pub mod prelude {
    pub use super::Action;
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum Action {
    Move,
    Zoom,
    Debug,

    EditorSelect,
    EditorCreate,
}

impl Actionlike for Action {
    // Record what kind of inputs make sense for each action.
    fn input_control_kind(&self) -> InputControlKind {
        match self {
            Self::Move => InputControlKind::DualAxis,
            Self::Zoom => InputControlKind::Axis,
            _ => InputControlKind::Button,
        }
    }
}

impl Action {
    fn default_input_map() -> InputMap<Self> {
        InputMap::default()
            .with_dual_axis(Self::Move, VirtualDPad::wasd())
            .with_axis(Self::Zoom, MouseScrollAxis::Y)
            .with(Self::Debug, KeyCode::KeyF)
            .with(
                Self::EditorCreate,
                ButtonlikeChord::from_single(MouseButton::Left).with(KeyCode::KeyQ),
            )
    }
}

app!(|app| {
    app.init_resource::<ActionState<Action>>()
        .insert_resource(Action::default_input_map());
});