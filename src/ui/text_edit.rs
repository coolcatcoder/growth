use bevy::input::{
    keyboard::{Key, KeyboardInput},
    ButtonState,
};

use crate::prelude::*;

pub mod prelude {
    pub use super::TextInput;
}

/// A simple text editing widget.
/// Assumed that it is always focused.
/// Modified from https://github.com/dothanhtrung/bevy-text-edit
#[derive(Component)]
#[require(Text)]
pub struct TextInput {
    // In characters.
    pub width_minimum: u8,
    pub cursor: u8,
    pub text: String,
}

impl TextInput {
    pub fn new(width_minimum: u8) -> Self {
        Self {
            width_minimum,
            cursor: 0,
            text: String::new(),
        }
    }
}

// We only expect 1 text editor to exist.
#[system(Update)]
fn keyboard_input(
    text_editor: Option<Single<(&mut TextInput, &mut Text)>>,
    mut input: EventReader<KeyboardInput>,
) {
    let Some(text_editor) = text_editor else {
        return;
    };

    let (mut editor, mut text) = text_editor.into_inner();

    let mut changed = false;

    if editor.is_added() {
        changed = true;
    }

    input.read().for_each(|input| {
        if input.state == ButtonState::Released {
            return;
        }

        match &input.logical_key {
            Key::ArrowLeft => {
                if editor.cursor > 0 {
                    changed = true;
                    editor.cursor -= 1;
                }
            }
            Key::ArrowRight => {
                let text_length = editor.text.chars().count();
                if text_length != 0 && (editor.cursor as usize) < text_length {
                    changed = true;
                    editor.cursor += 1;
                }
            }
            Key::Character(character) => {
                changed = true;
                let byte_index = if let Some(byte_index) =
                    editor.text.char_indices().nth(editor.cursor as usize)
                {
                    byte_index.0
                } else if editor.cursor == 0 {
                    // This should only happen when cursor is 0 and the text is empty.
                    0
                } else {
                    // The cursor should be at the very end then.
                    editor.text.len()
                };

                editor.text.insert_str(byte_index, &character);
                editor.cursor += 1;
            }
            Key::Backspace => {
                if editor.cursor > 0 {
                    changed = true;

                    let byte_index = editor
                        .text
                        .char_indices()
                        .nth(editor.cursor as usize - 1)
                        .unwrap()
                        .0;
                    editor.text.remove(byte_index);

                    editor.cursor -= 1;
                }
            }
            _ => (),
        }
    });

    if changed {
        let mut text_with_cursor_unpadded = editor.text.clone();
        if let Some((byte_index, _)) = text_with_cursor_unpadded
            .char_indices()
            .nth(editor.cursor as usize)
        {
            text_with_cursor_unpadded.insert(byte_index, '|');
        } else {
            // This branch should only happen when the cursor is at index 0 in an empty text.
            text_with_cursor_unpadded.push('|');
        }

        // Plus 1 to account for the cursor.
        let text_length = editor.text.len() + 1;

        if text_length >= editor.width_minimum as usize {
            text.0 = text_with_cursor_unpadded;
        } else {
            let padding_needed = editor.width_minimum as usize - text_length;
            let mut text_with_cursor =
                String::with_capacity(padding_needed + text_with_cursor_unpadded.len());

            for _ in 0..padding_needed {
                text_with_cursor.push('_');
            }

            text_with_cursor.insert_str(padding_needed / 2, &text_with_cursor_unpadded);

            text.0 = text_with_cursor;
        }
    }
}
