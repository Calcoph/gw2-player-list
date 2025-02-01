use std::{collections::HashMap, fs::File, io::Write, ops::DerefMut, sync::{Mutex, MutexGuard}};
use arcdps::{callbacks::{ImguiCallback, OptionsWindowsCallback}, exports, extras::{ExtrasAddonInfo, UserInfoIter}, imgui::{ColorEdit, Io, TableColumnSetup, Ui}};
use once_cell::sync::Lazy;
use toml::{map::Map, Value};
use windows::System::VirtualKey;

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
}

impl Flags {
    fn new() -> Flags {
        Flags {
            extras_initialized: false,
            display_window: false,
            show_all: false
        }
    }
}

struct State {
    players: PlayerVecMap,
    self_name: String,
    flags: Flags,
    filters: Filters,
    inactive_color: [f32;4],
    comment_size: [f32;2],
    add_user_text: String,
    shortcut_char: Option<VirtualKey>,
    listening_to_key: bool,
}

impl State {
    fn new() -> State {
        State {
            players: PlayerVecMap::new(),
            self_name: "".to_string(),
            flags: Flags::new(),
            filters: Filters::new(),
            inactive_color: DEFAULT_INACTIVE_COLOR,
            comment_size: DEFAULT_COMMENT_SIZE,
            add_user_text: "".to_string(),
            shortcut_char: None,
            listening_to_key: false
        }
    }
}

static mut STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::new()));
const CONFIG_PATH: &'static str = "addons/arcdps/player_list.toml";
const TMP_PATH: &'static str = "addons/arcdps/player_list.tmp";

const PLAYERS: &'static str = "Players";
const OPENED_WINDOW: &'static str = "WindowOpen";
const INACTIVE_COLOR: &'static str = "InactiveColor";
const SHOW_ALL: &'static str = "ShowAll";
const COMMENT_SIZE: &'static str = "CommentSize";
const DEFAULT_INACTIVE_COLOR: [f32;4] = [0.5,0.5,0.5,1.0];
const DEFAULT_COMMENT_SIZE: [f32;2] = [300.0, 20.0];
const SHORTCUT: &'static str = "ShortcutKey";

fn init() -> Result<(), String> {
    // May return an error to indicate load failure

    let toml_string = std::fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let mut config = match toml::from_str::<Value>(&toml_string)
        .unwrap_or(Value::Table(Map::new())) {
            Value::Table(config) => config,
            _ => Map::new()
        };

    let player_list = init_player_list(&mut config);
    let display_window = match config.remove(OPENED_WINDOW) {
        Some(Value::Boolean(b)) => b,
        _ => false,
    };
    let inactive_color = match config.remove(INACTIVE_COLOR) {
        Some(Value::Array(mut arr)) => {
            if arr.len() == 4 {
                let a = arr.remove(3);
                let b = arr.remove(2);
                let g = arr.remove(1);
                let r = arr.remove(0);
                if let (Value::Float(r), Value::Float(g), Value::Float(b), Value::Float(a)) = (r,g,b,a) {
                    [r as f32,g as f32,b as f32,a as f32]
                } else {
                    DEFAULT_INACTIVE_COLOR
                }
            } else {
                DEFAULT_INACTIVE_COLOR
            }
        },
        _ => DEFAULT_INACTIVE_COLOR,
    };
    let comment_size = match config.remove(COMMENT_SIZE) {
        Some(Value::Array(mut arr)) => {
            if arr.len() == 2 {
                let h = arr.remove(1);
                let w = arr.remove(0);
                if let (Value::Float(w), Value::Float(h)) = (w, h) {
                    [w as f32,h as f32]
                } else {
                    DEFAULT_COMMENT_SIZE
                }
            } else {
                DEFAULT_COMMENT_SIZE
            }
        },
        _ => DEFAULT_COMMENT_SIZE,
    };
    let show_all = match config.remove(SHOW_ALL) {
        Some(Value::Boolean(b)) => b,
        _ => false,
    };

    let shortcut_char = match config.remove(SHORTCUT) {
        Some(Value::String(s)) => { // For compatibility with 0.1.2
            if s.len() == 1 {
                let c = s.chars()
                    .next()
                    .filter(|c| ('A'..='Z').contains(c));
                match c {
                    Some(c) => match c {
                        'A' => Some(VirtualKey::A),
                        'B' => Some(VirtualKey::B),
                        'C' => Some(VirtualKey::C),
                        'D' => Some(VirtualKey::D),
                        'E' => Some(VirtualKey::E),
                        'F' => Some(VirtualKey::F),
                        'G' => Some(VirtualKey::G),
                        'H' => Some(VirtualKey::H),
                        'I' => Some(VirtualKey::I),
                        'J' => Some(VirtualKey::J),
                        'K' => Some(VirtualKey::K),
                        'L' => Some(VirtualKey::L),
                        'M' => Some(VirtualKey::M),
                        'N' => Some(VirtualKey::N),
                        'O' => Some(VirtualKey::O),
                        'P' => Some(VirtualKey::P),
                        'Q' => Some(VirtualKey::Q),
                        'R' => Some(VirtualKey::R),
                        'S' => Some(VirtualKey::S),
                        'T' => Some(VirtualKey::T),
                        'U' => Some(VirtualKey::U),
                        'V' => Some(VirtualKey::V),
                        'W' => Some(VirtualKey::W),
                        'X' => Some(VirtualKey::X),
                        'Y' => Some(VirtualKey::Y),
                        'Z' => Some(VirtualKey::Z),
                        _ => None
                    },
                    None => None,
                }
            } else {
                None
            }
        },
        Some(Value::Integer(i)) => {
            Some(VirtualKey(i as i32))
        }
        _ => None
    };

    let mut state = get_state();
    state.players = player_list;
    state.flags.display_window = display_window;
    state.flags.show_all = show_all;
    state.inactive_color = inactive_color;
    state.comment_size = comment_size;
    state.shortcut_char = shortcut_char;

    Ok(())
}

fn init_extras(_: ExtrasAddonInfo, self_name: Option<&str>) {
    let mut state = get_state();

    if let Some(self_name) = self_name {
        state.flags.extras_initialized = true;
        state.self_name = self_name.to_owned();
    }
}

fn init_player_list(config: &mut Map<String, Value>) -> PlayerVecMap {
    let players = config.remove(PLAYERS);

    let players = match players {
        Some(Value::Array(players)) => players,
        _ => vec![],
    };

    let mut player_map = HashMap::new();

    let player_list: Vec<_> = players.into_iter()
        .filter_map(|val| {
            let mut properties = match val {
                Value::Table(properties) => properties,
                _ => return None
            };

            let name = properties.remove("name");
            let comment = properties.remove("comment");

            if let (Some(Value::String(name)), Some(Value::String(comment))) = (name, comment) {
                Some(Player {
                    lowercase_name: name.to_lowercase(),
                    name,
                    lowercase_comment: comment.to_lowercase(),
                    comment,
                    in_squad: false,
                })
            } else {
                None
            }
        }).collect();

    for (i, player) in player_list.iter().enumerate() {
        player_map.insert(player.name.clone(), i);
    }

    PlayerVecMap {
        player_list,
        name_dict: player_map,
    }
}

fn release() {
    let mut config = Map::new();

    let state = get_state();
    let player_list = state.players.player_list.iter().filter_map(|player| {
        if player.comment != "" {
            Some(player.to_toml())
        } else {
            None
        }
    }).collect();
    config.insert(PLAYERS.to_string(), Value::Array(player_list));
    config.insert(OPENED_WINDOW.to_string(), Value::Boolean(state.flags.display_window));
    let inactive_color = state.inactive_color.into_iter()
        .map(|val| Value::Float(val as f64)).collect();
    config.insert(INACTIVE_COLOR.to_string(), Value::Array(inactive_color));
    let comment_size = state.comment_size.into_iter()
        .map(|val| Value::Float(val as f64)).collect();
    config.insert(COMMENT_SIZE.to_string(), Value::Array(comment_size));
    config.insert(SHOW_ALL.to_string(), Value::Boolean(state.flags.show_all));
    if let Some(i) = state.shortcut_char {
        config.insert(SHORTCUT.to_string(), Value::Integer(i.0 as i64));
    }

    let toml_string = toml::to_string(&Value::Table(config)).unwrap();
    std::fs::write(CONFIG_PATH, toml_string).unwrap()
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
        arcdps::imgui::Window::new("Player List Error").collapsible(false).build(ui, || {
            ui.text("Unofficial extras extension required")
        });

        return
    };

    let mut opened_window = state.flags.display_window;
    std::mem::drop(state); // liberates the mutex so get_state() can be called again from the closure in .build()
    if opened_window {
        arcdps::imgui::Window::new("Player List").opened(&mut opened_window).collapsible(false).build(ui, || {
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
                        ui.text(&player.name);
                    } else {
                        ui.text_colored(state.inactive_color, &player.name)
                    }

                    ui.table_next_column();
                    if ui.input_text_multiline(format!("##{i}"), &mut player.comment, state.comment_size).build() {
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
    ColorEdit::new("Inactive player", &mut state.inactive_color).build(ui);
    if ui.is_item_hovered() {
        ui.tooltip_text("Color of the names of players out of the squad")
    }

    ui.input_float2("Comment Size", &mut state.comment_size).build();

    match state.shortcut_char {
        Some(c) => ui.text(format!("Shortcut: {}", vk_to_text(c))),
        None => ui.text("No shortcut set"),
    }

    ui.same_line();
    if ui.button("X") {
        state.shortcut_char = None
    }

    if state.listening_to_key {
        ui.same_line();
        ui.text("Listening ... ");
        ui.same_line();
        if ui.button("Cancel") {
            state.listening_to_key = false;
            state.shortcut_char = None
        }
    } else {
        ui.same_line();
        if ui.button("Set shortcut") {
            state.listening_to_key = true
        }
    }
}

fn log(msg: &str) {
    writeln!(File::options().create(true).append(true).open(TMP_PATH).unwrap(), "{msg}").unwrap();
}

fn shortcuts(key: usize, key_down: bool, holding_key: bool) -> bool {
    let mut state = get_state();
    if key_down && !holding_key {
        // Both modifier keys have been pressed
        // modifiers are alt+shift by default
        if let Some(c) = state.shortcut_char {
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
    if key_down && !holding_key && state.listening_to_key {
        state.listening_to_key = false;
        state.shortcut_char = Some(VirtualKey(key as i32));
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
