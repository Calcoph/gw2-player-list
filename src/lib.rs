#![allow(static_mut_refs)]
const VERSION_MAJOR: u32 = 1;
const VERSION_MINOR: u32 = 0;
const VERSION_PATCH: u32 = 0;

use std::{collections::HashMap, ops::DerefMut, sync::{Mutex, MutexGuard}};
use arcdps::{extras::{ExtrasAddonInfo, UserInfoIter}, imgui::{InputTextFlags, TableColumnSetup, Ui}};
use const_format::formatcp;
use once_cell::sync::Lazy;
use toml::{map::Map, Value};
use windows::System::VirtualKey;

use crate::config::Config;

mod config;

arcdps::export! {
    name: "Player List",
    sig: 0x73242FB, // random number
    init,
    extras_init: init_extras,
    release,
    imgui: draw_window,
    extras_squad_update: squad_update,
    options_windows: options,
    options_end: options_tab,
    wnd_filter: shortcuts,
    wnd_nofilter: nofilter,
    update_url: check_for_updates
}

struct Player {
    name: String,
    lowercase_name: String,
    comment: String,
    lowercase_comment: String,
    in_squad: bool
}

impl Player {
    fn to_toml(&self) -> Value {
        let mut toml_map = Map::new();

        toml_map.insert("name".to_string(), Value::String(self.name.clone()));
        toml_map.insert("comment".to_string(), Value::String(self.comment.clone()));

        Value::Table(toml_map)
    }
}

struct PlayerVecMap {
    player_list: Vec<Player>,
    name_dict: HashMap<String, usize>
}

impl PlayerVecMap {
    fn new() -> PlayerVecMap {
        PlayerVecMap {
            player_list: Vec::new(),
            name_dict: HashMap::new()
        }
    }

    fn is_deletable(&self, username: &str) -> bool {
        let mut delete = false;
        if let Some(idx) = self.name_dict.get(username) {
            if let Some(player) = self.player_list.get(*idx) {
                // Only delete if there is no comment
                delete = player.comment == ""
            }
        };

        delete
    }

    fn user_left(&mut self, username: &str) {
        let delete = self.is_deletable(username);
        if delete {
            let index = self.name_dict.remove(username).unwrap();
            self.delete_at(index)
        }

        if let Some(index) = self.name_dict.get(username) {
            let player = &mut self.player_list[*index];
            player.in_squad = false;
        }
    }

    /// deletes ONLY from self.player_list. Use delete() to also delete from self.name_dict
    fn delete_at(&mut self, index: usize) {
        self.player_list.remove(index);

        // After deleting the elements in the vec, all elements after it are shifted to the left. Update the indices
        for (_, idx) in self.name_dict.iter_mut() {
            if *idx > index {
                *idx -= 1
            }
        }
    }

    /// deletes from BOTH self.player_list and self.name_dict. Use delete_at() to only delete from self.player_list
    fn delete(&mut self, username: &str) {
        if let Some(index) = self.name_dict.remove(username) {
            self.player_list.remove(index);

            // After deleting the elements in the vec, all elements after it are shifted to the left. Update the indices
            for (_, idx) in self.name_dict.iter_mut() {
                if *idx > index {
                    *idx -= 1
                }
            }
        };
    }

    /// Deletes all players whose comment is an empty string
    fn delete_all(&mut self) {
        let mut delete_list = Vec::new();

        // The indices will be in reverse order so we can delete
        // them in same order without shifting any to-delete elements
        for player in self.player_list.iter_mut().rev() {
            player.in_squad = false;
            if player.comment == "" {
                if let Some(idx) = self.name_dict.remove(&player.name) {
                    delete_list.push(idx)
                }
            }
        }

        for idx in delete_list {
            self.delete_at(idx)
        }
    }

    fn join(&mut self, username: &str) {
        self.add_player(username, "".to_string());

        if let Some(index) = self.name_dict.get(username) {
            let player = &mut self.player_list[*index];
            player.in_squad = true;
        };
    }

    fn add_player(&mut self, username: &str, comment: String) {
        let add = !self.name_dict.contains_key(username);
        if add {
            let new_item_index = self.player_list.len();
            self.name_dict.insert(username.to_string(), new_item_index);
            self.player_list.push(Player {
                name: username.to_string(),
                lowercase_name: username.to_lowercase(),
                comment,
                lowercase_comment: "".to_string(),
                in_squad: false
            });
        }
    }
}

struct Filters {
    user_filter_str: String,
    comment_filter_str: String
}

impl Filters {
    fn new() -> Filters {
        Filters {
            user_filter_str: String::new(),
            comment_filter_str: String::new()
        }
    }
}

struct Flags {
    extras_initialized: bool,
    display_window: bool,
    show_all: bool,
    listening_to_key: bool,
}

impl Flags {
    fn new() -> Flags {
        Flags {
            extras_initialized: false,
            display_window: false,
            show_all: false,
            listening_to_key: false,
        }
    }
}

struct State {
    players: PlayerVecMap,
    self_name: String,
    flags: Flags,
    filters: Filters,
    add_user_text: String,
    config: Config,
}

static mut STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(config::default_state()));


fn init() -> Result<(), Option<String>> {
    // May return an error to indicate load failure

    config::parse(&mut get_state())?;

    #[cfg(debug_assertions)] // In order to work with arcdps_mock
    {
        let state = get_state();
        if !state.flags.extras_initialized {
            extras_initializer(state, Some("abcdtest"));
        }
    }

    Ok(())
}

fn extras_initializer(mut state: MutexGuard<'_, State>, self_name: Option<&str>) {
    if let Some(self_name) = self_name {
        state.flags.extras_initialized = true;
        state.self_name = self_name.to_owned();
    } else {
        #[cfg(debug_assertions)]
        {
            state.flags.extras_initialized = false;
        }
    }
}

fn init_extras(_: ExtrasAddonInfo, self_name: Option<&str>) {
    let state = get_state();
    extras_initializer(state, self_name);
}

fn release() {
    let mut state = get_state();
    if let Err(_) = config::save(&mut state) {
        log("Failed to save!")
    };
}

fn get_state<'a>() -> MutexGuard<'a, State>{
    unsafe{STATE.lock().unwrap()}
}

fn squad_update(users: UserInfoIter) {
    for user in users {
        if let Some(username) = user.account_name() {
            match user.role {
                arcdps::extras::UserRole::None => remove_user(username),
                _ => add_user(username),
            }
        }
    }
}

fn remove_user(username: &str) {
    let mut state = get_state();

    let is_self = username == state.self_name;

    if is_self {
        state.players.delete_all()
    } else {
        state.players.user_left(username);
    }
}

fn add_user(username: &str) {
    let mut state = get_state();

    let is_self = username == state.self_name;

    if !is_self {
        state.players.join(username);
    }
}

fn draw_window(ui: &Ui, not_character_or_loading: bool) {
    let state = get_state();
    if !not_character_or_loading {
        // Don't draw anything on character screen or loading screen
        return
    }

    if !state.flags.extras_initialized {
        ui.window("Player List Error").collapsible(false).build(|| {
            ui.text("Unofficial extras extension required")
        });

        return
    };

    let mut opened_window = state.flags.display_window;
    std::mem::drop(state); // liberates the mutex so get_state() can be called again from the closure in .build()
    if opened_window {
        ui.window("Player List").opened(&mut opened_window).collapsible(false).build(|| {
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
            {
                let mut state = get_state();
                let state = state.deref_mut();
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
            }
            let mut action = None;
            if let Some(table) = ui.begin_table_header("PLayerListTable", column_data) {
                let mut state = get_state();
                let state = state.deref_mut();
                let filters = &state.filters;
                let players = &mut state.players;
                for (i, player) in players.player_list.iter_mut().enumerate() {
                    if !filters.user_filter_str.is_empty() && !player.lowercase_name.starts_with(&filters.user_filter_str) {
                        continue;
                    }
                    if !filters.comment_filter_str.is_empty() && !player.lowercase_comment.starts_with(&filters.comment_filter_str) {
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
                    Action::DeletePlayer(username) => get_state().players.delete(&username),
                }
            }
        });
    }

    get_state().flags.display_window = opened_window;
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

enum Action {
    DeletePlayer(String)
}

fn options(ui: &Ui, window_name: Option<&str>) -> bool {
    if let Some("error") = window_name {
        ui.checkbox("player list", &mut get_state().flags.display_window);
    }

    false
}

fn options_tab(ui: &Ui) {
    let mut state = get_state();
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

// log only does something in debug builds
fn log(msg: &str) {
    let _ = msg;
    #[cfg(debug_assertions)]
    {
        use std::io::Write;
        writeln!(std::fs::File::options().create(true).append(true).open(config::TMP_PATH).unwrap(), "{msg}").unwrap();
    }
}

fn shortcuts(key: usize, key_down: bool, holding_key: bool) -> bool {
    let mut state = get_state();
    if key_down && !holding_key {
        // Both modifier keys have been pressed
        // modifiers are alt+shift by default
        if let Some(c) = state.config.shortcut_char {
            if key == c.0 as usize {
                state.flags.display_window = !state.flags.display_window;
                return false
            }
        }
    }

    true
}

fn nofilter(key: usize, key_down: bool, holding_key: bool) -> bool {
    let mut state = get_state();
    if key_down && !holding_key && state.flags.listening_to_key {
        state.flags.listening_to_key = false;
        state.config.shortcut_char = Some(VirtualKey(key as i32));
        return false
    }

    true
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

fn check_for_updates() -> Option<String> {
    const RELEASE_LIST_LEN: u32 = 5; // check at most the most recent RELEASE_LIST_LEN updates
    const URL: &'static str = formatcp!("https://api.github.com/repos/Calcoph/gw2-player-list/releases?per_page={RELEASE_LIST_LEN}&page=1");

    const USER_AGENT: &'static str = formatcp!("gw2_player_list_{VERSION_MAJOR}_{VERSION_MINOR}_{VERSION_PATCH}");

    let state = get_state();
    if !state.config.auto_check_update {
        return None;
    }

    let http_client = reqwest::blocking::ClientBuilder::new()
        .user_agent(USER_AGENT)
        .build().ok()?;

    let response = http_client.get(URL).send().ok()?; // TODO: Do not spam the api, do not check for updates every time gw2 is launched. Maybe once a week or so. Also use etags https://docs.github.com/en/rest/using-the-rest-api/best-practices-for-using-the-rest-api?apiVersion=2026-03-10#make-requests-that-can-be-cached

    let status = response.status();
    let body = response.text().ok()?;
    if !status.is_success() {
        log(&format!("Update error ({}) response body: {body}", status.as_u16()));
        return None;
    }

    let ret: serde_json::Value = serde_json::from_str(&body).ok()?;
    let serde_json::Value::Array(releases) = ret else {
        return None;
    };

    let mut chosen_release = None;
    'outer: for release in releases {
        let serde_json::Value::Object(release) = release else {
            continue;
        };

        if !state.config.auto_check_beta {
            if let Some(serde_json::Value::Bool(true)) = release.get("prerelease") {
                continue;
            }
        }

        let Some(serde_json::Value::Array(assets)) = release.get("assets") else {
            continue;
        };

        let Some(serde_json::Value::String(tag)) = release.get("tag_name") else {
            continue;
        };

        let Some(version) = parse_tag(tag) else {
            continue;
        };

        if !is_version_newer(version) {
            continue;
        }

        for asset in assets {
            let Some(serde_json::Value::String(name)) = asset.get("name") else {
                continue;
            };

            if name != "player_list.dll" {
                continue;
            }

            let Some(serde_json::Value::String(url)) = asset.get("browser_download_url") else {
                continue;
            };

            log(&format!("Updating version to {}.{}.{}", version.0, version.1, version.2));
            chosen_release = Some(url.clone());
            break 'outer;
        }
    }

    if chosen_release.is_none() {
        log("No new version has been detected");
    }
    chosen_release
}

fn is_version_newer((major, minor, patch): (u32, u32, u32)) -> bool {
    if major > VERSION_MAJOR {
        return true;
    }
    if major < VERSION_MAJOR {
        return false;
    }

    if minor > VERSION_MINOR {
        return true;
    }
    if minor < VERSION_MINOR {
        return false;
    }

    return patch > VERSION_PATCH;
}

fn parse_tag(tag: &str) -> Option<(u32, u32, u32)> {
    if !tag.starts_with("v") {
        return None;
    }

    let tag = tag.trim_start_matches("v");

    let mut parts = tag.split(".");
    let major = parts.next()?;
    let minor = parts.next()?;
    let patch = parts.next()?;

    let patch = patch.split("-").next()?;

    let major = major.parse().ok()?;
    let minor = minor.parse().ok()?;
    let patch = patch.parse().ok()?;

    Some((major, minor, patch))
}
