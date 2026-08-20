//! Key events, lifted out of the C enums into ones that can be matched
//! exhaustively.

use flipperzero_sys as sys;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    Ok,
    Back,
}

/// What happened to the key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    /// Pushed down.
    Down,
    /// Released.
    Up,
    /// Pushed and released inside the short-press window.
    Short,
    /// Held past the long-press threshold.
    Long,
    /// Auto-repeat while held.
    Repeat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputEvent {
    pub key: Key,
    pub press: Press,
}

impl InputEvent {
    /// `None` for keys and gestures the firmware knows about but this wrapper
    /// does not, so an unexpected variant is ignored rather than quietly mapped
    /// onto the wrong one.
    pub(crate) fn from_sys(event: &sys::InputEvent) -> Option<Self> {
        Some(Self {
            key: key_from_sys(event.key)?,
            press: press_from_sys(event.type_)?,
        })
    }
}

fn key_from_sys(key: sys::InputKey) -> Option<Key> {
    Some(match key {
        sys::InputKeyUp => Key::Up,
        sys::InputKeyDown => Key::Down,
        sys::InputKeyLeft => Key::Left,
        sys::InputKeyRight => Key::Right,
        sys::InputKeyOk => Key::Ok,
        sys::InputKeyBack => Key::Back,
        _ => return None,
    })
}

fn press_from_sys(press: sys::InputType) -> Option<Press> {
    Some(match press {
        sys::InputTypePress => Press::Down,
        sys::InputTypeRelease => Press::Up,
        sys::InputTypeShort => Press::Short,
        sys::InputTypeLong => Press::Long,
        sys::InputTypeRepeat => Press::Repeat,
        _ => return None,
    })
}
