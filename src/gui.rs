use arcdps::imgui::{InputTextFlags, TableColumnSetup, Ui};
use const_format::formatcp;
use windows::System::VirtualKey;

use crate::{Action, State, VERSION_MAJOR, VERSION_MINOR, VERSION_PATCH, update_checker};

pub fn main_window(ui: &Ui, state: &mut State) {
    if !state.flags.extras_initialized {
        ui.window("Player List Error").collapsible(false).build(|| {
            ui.text("Unofficial extras extension required")
        });
        return
    };

    if !state.flags.config_correctly_parsed {
        ui.window("Player List Error").collapsible(false).build(|| {
            ui.text("Could not read configuration file")
        });
        return
    };

    if let Some(available_version) = state.updater_data.available_version.as_ref() {
        if state.config.auto_check_update && !state.flags.ignore_updater_window {
            let mut update = false;
            ui.window("Player List Updater").collapsible(false).build(|| {
                ui.text("A new update for Player List is available");
                ui.text("The update will be downloaded from:");
                ui.indent();
                ui.text(&available_version.url);
                ui.unindent();
                ui.text(formatcp!("Current version: {VERSION_MAJOR}.{VERSION_MINOR}.{VERSION_PATCH}"));
                ui.text(format!("Downloadable version: {}.{}.{}", available_version.major, available_version.minor, available_version.patch));
                ui.text("Download on next login?");

                if ui.button("Yes") {
                    update = true;
                }
                ui.same_line();
                if ui.button("Not now") {
                    state.flags.ignore_updater_window = true;
                }
                ui.same_line();
                if ui.button("Disable auto updates") {
                    state.config.auto_check_update = false;
                }
            });
            if update {
                update_checker::update(state);
            }
            return; // Do not display main window until user makes a choice on thee update to not clutter the screen with windows
        }
    }

    let mut opened_window = state.flags.display_window;

    if opened_window {
        ui.window("Player List").opened(&mut opened_window).collapsible(false).build(|| main_window_impl(ui, state));
    }

    state.flags.display_window = opened_window;
}

fn main_window_impl(ui: &Ui, state: &mut State) {
    let column_data = [
        // max character length of account name = 32 characters
        TableColumnSetup {
            name: "name",
            ..Default::default()
        },
        TableColumnSetup {
            name: "comment",
            ..Default::default()
        }
    ];

    ui.checkbox("Show all", &mut state.flags.show_all);

    ui.separator();
    ui.text("Add user:");
    ui.input_text("##add_user", &mut state.add_user_text).build();
    ui.same_line();
    if ui.button("Add") {
        if !state.add_user_text.is_empty() {
            state.players.add_player(&state.add_user_text, "Comment here".to_string());
            state.add_user_text = "".to_string();
        }
    };

    ui.separator();
    ui.text("Filters:");
    if ui.input_text("##user_filter", &mut state.filters.user_filter_str).build() {
        state.filters.user_filter_str = state.filters.user_filter_str.to_lowercase()
    };
    if ui.is_item_hovered() {
        ui.tooltip_text("Filter by user name")
    }
    if ui.input_text("##comment_filter", &mut state.filters.comment_filter_str).build() {
        state.filters.comment_filter_str = state.filters.comment_filter_str.to_lowercase()
    };
    if ui.is_item_hovered() {
        ui.tooltip_text("Filter by comment")
    }

    let mut action = None;
    if let Some(table) = ui.begin_table_header("PLayerListTable", column_data) {
        let filters = &state.filters;
        let players = &mut state.players;
        for (i, player) in players.player_list.iter_mut().enumerate() {
            if !filters.user_filter_str.is_empty() && !player.lowercase_name.contains(&filters.user_filter_str) {
                continue;
            }
            if !filters.comment_filter_str.is_empty() && !player.lowercase_comment.contains(&filters.comment_filter_str) {
                continue;
            }
            if !state.flags.show_all && !player.in_squad {
                continue;
            }
            ui.table_next_column();
            if ui.button(format!("X##delete_{i}")) {
                action = Some(Action::DeletePlayer(player.name.clone()))
            }
            if ui.is_item_hovered() {
                ui.tooltip_text("Delete this player\nfrom the list")
            }
            ui.same_line();
            if player.in_squad {
                text(ui, &mut player.name);
            } else {
                text_colored(ui, state.config.inactive_color, &mut player.name);
            }

            ui.table_next_column();
            if ui.input_text_multiline(format!("##{i}"), &mut player.comment, state.config.comment_size).build() {
                player.lowercase_comment = player.comment.to_lowercase()
            };
        }
        table.end()
    };

    if let Some(action) = action {
        match action {
            Action::DeletePlayer(username) => state.players.delete(&username),
        }
    }
}

fn text(ui: &Ui, txt: &mut String) {
    let tok = ui.push_style_color(arcdps::imgui::StyleColor::FrameBg, [0.0,0.0,0.0,0.0]);
    ui.input_text(format!("##selectable_text{txt}"), txt)
        .flags(InputTextFlags::READ_ONLY)
        .build();
    tok.end();
}

fn text_colored(ui: &Ui, color: [f32;4], txt: &mut String) {
    let tok = ui.push_style_color(arcdps::imgui::StyleColor::Text, color);
    let tok2 = ui.push_style_color(arcdps::imgui::StyleColor::FrameBg, [0.0,0.0,0.0,0.0]);
    ui.input_text(format!("##selectable_text{txt}"), txt)
        .flags(InputTextFlags::READ_ONLY)
        .build();
    tok2.end();
    tok.end();
}

pub fn arcdps_options(ui: &Ui, state: &mut State) {
    ui.checkbox("player list", &mut state.flags.display_window);
}

pub fn options(ui: &Ui, state: &mut State) {
    if !state.flags.config_correctly_parsed {
        ui.text("Could not read configuration file");
        return
    };

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
    ui.text(formatcp!("Current version: {VERSION_MAJOR}.{VERSION_MINOR}.{VERSION_PATCH}"));
    ui.separator();
    ui.checkbox("Enable", &mut state.config.auto_check_update);
    ui.checkbox("Allow beta releases", &mut state.config.auto_check_beta);
    ui.text("Check for updates every");
    ui.same_line();
    if ui.input_int("days", &mut state.updater_data.days_between_polls).build() {
        if state.updater_data.days_between_polls < 0 {
            state.updater_data.days_between_polls = 0;
        }
    }
    if ui.button("Check for updates on next login") {
        state.updater_data.reset()
    }
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
