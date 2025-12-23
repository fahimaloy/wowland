use serde::Deserialize;
use smithay::input::keyboard::{keysyms, xkb, Keysym, KeysymHandle, ModifiersState};

use crate::compositor::window::WindowId;

const NO_SYMBOL: Keysym = Keysym::new(keysyms::KEY_NoSymbol);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Action {
    Quit,
    NextLayout,
    PrevLayout,
    FocusNext,
    FocusPrev,
    ToggleFloat,
    ToggleMaximize,
    ToggleMinimize,
    CloseFocused,
    CycleOpacity,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct ModifiersMask {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub super_key: bool,
}

impl ModifiersMask {
    pub fn matches(&self, mods: &ModifiersState, super_is_alt: bool) -> bool {
        let super_pressed = if super_is_alt { mods.alt } else { mods.logo };
        (!self.ctrl || mods.ctrl)
            && (!self.alt || mods.alt)
            && (!self.shift || mods.shift)
            && (!self.super_key || super_pressed)
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedKeybinding {
    pub action: Action,
    pub keysym: Keysym,
    pub modifiers: ModifiersMask,
}

#[derive(Debug, Default)]
pub struct InputState {
    keybindings: Vec<ResolvedKeybinding>,
    modifiers: ModifiersState,
    pointer_location: (f64, f64),
    drag_state: Option<DragState>,
    resize_state: Option<ResizeState>,
    super_is_alt: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct DragState {
    pub window_id: WindowId,
    pub offset: (f64, f64),
}

#[derive(Debug, Clone, Copy)]
pub struct ResizeState {
    pub window_id: WindowId,
    pub start_pointer: (f64, f64),
    pub start_size: (i32, i32),
}

impl InputState {
    pub fn new(bindings: Vec<ResolvedKeybinding>, super_is_alt: bool) -> Self {
        Self {
            keybindings: bindings,
            modifiers: ModifiersState::default(),
            pointer_location: (0.0, 0.0),
            drag_state: None,
            resize_state: None,
            super_is_alt,
        }
    }

    pub fn update_modifiers(&mut self, mods: &ModifiersState) {
        self.modifiers = *mods;
    }

    pub fn modifiers(&self) -> ModifiersState {
        self.modifiers
    }

    pub fn update_pointer_location(&mut self, x: f64, y: f64) {
        self.pointer_location = (x, y);
    }

    pub fn pointer_location(&self) -> (f64, f64) {
        self.pointer_location
    }

    pub fn drag_state(&self) -> Option<DragState> {
        self.drag_state
    }

    pub fn begin_drag(&mut self, window_id: WindowId, offset: (f64, f64)) {
        self.drag_state = Some(DragState { window_id, offset });
    }

    pub fn end_drag(&mut self) {
        self.drag_state = None;
    }

    pub fn resize_state(&self) -> Option<ResizeState> {
        self.resize_state
    }

    pub fn begin_resize(&mut self, window_id: WindowId, start_pointer: (f64, f64), start_size: (i32, i32)) {
        self.resize_state = Some(ResizeState {
            window_id,
            start_pointer,
            start_size,
        });
    }

    pub fn end_resize(&mut self) {
        self.resize_state = None;
    }

    pub fn action_for(&self, mods: &ModifiersState, keysym: Keysym) -> Option<Action> {
        self.keybindings
            .iter()
            .find(|binding| binding.keysym == keysym && binding.modifiers.matches(mods, self.super_is_alt))
            .map(|binding| binding.action)
    }
}

pub fn resolve_keybindings(bindings: &[crate::compositor::config::KeybindingConfig]) -> Vec<ResolvedKeybinding> {
    bindings
        .iter()
        .filter_map(|binding| {
            let keysym = resolve_keysym(&binding.key)?;
            let keysym = normalize_keysym(keysym);
            let modifiers = parse_modifiers(&binding.modifiers);
            Some(ResolvedKeybinding {
                action: binding.action,
                keysym,
                modifiers,
            })
        })
        .collect()
}

pub fn key_from_handle(handle: &KeysymHandle<'_>) -> Option<Keysym> {
    if let Some(sym) = handle.raw_latin_sym_or_raw_current_sym() {
        if sym != NO_SYMBOL {
            return Some(normalize_keysym(sym));
        }
    }
    let sym = handle.modified_sym();
    if sym == NO_SYMBOL {
        None
    } else {
        Some(normalize_keysym(sym))
    }
}

fn resolve_keysym(name: &str) -> Option<Keysym> {
    let sym = xkb::keysym_from_name(name, xkb::KEYSYM_NO_FLAGS);
    if sym != NO_SYMBOL {
        return Some(sym);
    }
    let sym = xkb::keysym_from_name(name, xkb::KEYSYM_CASE_INSENSITIVE);
    if sym == NO_SYMBOL {
        None
    } else {
        Some(sym)
    }
}

fn normalize_keysym(sym: Keysym) -> Keysym {
    if let Some(ch) = sym.key_char() {
        if ch.is_ascii_alphabetic() {
            return Keysym::new(ch.to_ascii_lowercase() as u32);
        }
    }
    sym
}

fn parse_modifiers(values: &[String]) -> ModifiersMask {
    let mut mask = ModifiersMask::default();
    for value in values {
        match value.to_lowercase().as_str() {
            "ctrl" | "control" => mask.ctrl = true,
            "alt" => mask.alt = true,
            "shift" => mask.shift = true,
            "super" | "logo" | "meta" => mask.super_key = true,
            _ => {}
        }
    }
    mask
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_match_when_required_set() {
        let mut mods = ModifiersState::default();
        mods.logo = true;
        let mask = ModifiersMask {
            super_key: true,
            ..ModifiersMask::default()
        };
        assert!(mask.matches(&mods, false));
    }
}
