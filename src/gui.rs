use arcdps::imgui::Ui;
use const_format::formatcp;
use windows::System::VirtualKey;

use crate::{State, VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH};

pub fn arcdps_options(ui: &Ui, state: &mut State) {
    ui.checkbox("player list", &mut state.flags.display_window);
}

pub fn options(ui: &Ui, state: &mut State) {
    ui.color_edit4("Inactive player", &mut state.config.inactive_color);
    if ui.is_item_hovered() {
        ui.tooltip_text("Color of the names of players out of the squad")
    }

    ui.input_float2("Comment Size", &mut state.config.comment_size).build();

    match state.config.shortcut_char {
        Some(c) => ui.text(format!("Shortcut: {}", vk_to_text(c))),
        None => ui.text("No shortcut set"),
    }

    ui.same_line();
    if ui.button("X") {
        state.config.shortcut_char = None
    }

    if state.flags.listening_to_key {
        ui.same_line();
        ui.text("Listening ... ");
        ui.same_line();
        if ui.button("Cancel") {
            state.flags.listening_to_key = false;
            state.config.shortcut_char = None
        }
    } else {
        ui.same_line();
        if ui.button("Set shortcut") {
            state.flags.listening_to_key = true
        }
    }

    ui.text("Automatic update configuration");
    ui.text(formatcp!("{VERSION_MAJOR}.{VERSION_MINOR}.{VERSION_PATCH}"));
    ui.separator();
    ui.checkbox("Enable", &mut state.config.auto_check_update);
    ui.checkbox("Allow beta releases", &mut state.config.auto_check_beta);
}

fn vk_to_text(vk: VirtualKey) -> String {
    match vk {
        VirtualKey::A => "A".to_string(),
        VirtualKey::B => "B".to_string(),
        VirtualKey::C => "C".to_string(),
        VirtualKey::D => "D".to_string(),
        VirtualKey::E => "E".to_string(),
        VirtualKey::F => "F".to_string(),
        VirtualKey::G => "G".to_string(),
        VirtualKey::H => "H".to_string(),
        VirtualKey::I => "I".to_string(),
        VirtualKey::J => "J".to_string(),
        VirtualKey::K => "K".to_string(),
        VirtualKey::L => "L".to_string(),
        VirtualKey::M => "M".to_string(),
        VirtualKey::N => "N".to_string(),
        VirtualKey::O => "O".to_string(),
        VirtualKey::P => "P".to_string(),
        VirtualKey::Q => "Q".to_string(),
        VirtualKey::R => "R".to_string(),
        VirtualKey::S => "S".to_string(),
        VirtualKey::T => "T".to_string(),
        VirtualKey::U => "U".to_string(),
        VirtualKey::V => "V".to_string(),
        VirtualKey::W => "W".to_string(),
        VirtualKey::X => "X".to_string(),
        VirtualKey::Y => "Y".to_string(),
        VirtualKey::Z => "Z".to_string(),
        VirtualKey(key) => format!("Key<{key}>")
    }
}
