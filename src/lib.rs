use std::{error::Error, collections::HashMap, sync::{Mutex, MutexGuard}, io::Write, fs::File};
use arcdps::{Agent, CombatEvent, StateChange, extras::{UserInfoIter, UserInfo, ExtrasAddonInfo}, callbacks::{ImguiCallback, OptionsWindowsCallback}, imgui::{Ui, TableColumnSetup, Id, TableColumnFlags}};
use once_cell::sync::Lazy;
use toml::{map::Map, Value};

arcdps::export! {
    name: "Player List",
    sig: 0x73242FB, // random number
    init,
    extras_init: init_extras,
    release,
    imgui: draw_window,
    extras_squad_update: squad_update,
    options_windows: options,
    // options_end, // This gives our own tab in the extension options
}

struct Player {
    name: String,
    comment: String
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

    fn delete_user(&mut self, username: &str) {
        let index = self.name_dict.remove(username).unwrap();
        self.delete_at(index)
    }
    
    fn delete_at(&mut self, index: usize) {
        self.player_list.remove(index);
        
        // After deleting the elements in the vec, all elements after it are shifted to the left. Update the indices
        for (_, idx) in self.name_dict.iter_mut() {
            if *idx > index {
                *idx -= 1
            }
        }
    }

    /// Deletes all players whose comment is an empty string
    fn delete_all(&mut self) {
        let mut delete_list = Vec::new();

        // The indices will be in reverse order so we can delete
        // them in same order without shifting any to-delete elements
        for player in self.player_list.iter().rev() {
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

    fn add(&mut self, username: &str) {
        let new_item_index = self.player_list.len();
        self.name_dict.insert(username.to_string(), new_item_index);
        self.player_list.push(Player {
            name: username.to_string(),
            comment: "".to_string()
        });
    }
}

struct State {
    players: PlayerVecMap,
    self_name: String,
    extras_initialized: bool,
    display_window: bool,
}

impl State {
    fn new() -> State {
        State {
            players: PlayerVecMap::new(),
            self_name: "".to_string(),
            extras_initialized: false,
            display_window: false
        }
    }
}

static mut STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(State::new()));
const CONFIG_PATH: &'static str = "addons/arcdps/player_list.toml";
const TMP_PATH: &'static str = "addons/arcdps/player_list.tmp";

const PLAYERS: &'static str = "Players";
const OPENED_WINDOW: &'static str = "WindowOpen";

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

    let mut state = get_state();
    state.players = player_list;
    state.display_window = display_window;

    Ok(())
}

fn init_extras(_: ExtrasAddonInfo, self_name: Option<&str>) {
    let mut state = get_state();

    if let Some(self_name) = self_name {
        state.extras_initialized = true;
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
                    name,
                    comment
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
    config.insert(OPENED_WINDOW.to_string(), Value::Boolean(state.display_window));

    let toml_string = toml::to_string(&Value::Table(config)).unwrap();
    std::fs::write(CONFIG_PATH, toml_string).unwrap()
}

fn get_state<'a>() -> MutexGuard<'a, State>{
    unsafe{STATE.lock().unwrap()}
}

fn squad_update(users: UserInfoIter) {
    for user in users {
        if let Some(username) = user.account_name {
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
    let delete = state.players.is_deletable(username);

    if is_self {
        state.players.delete_all()
    } else if delete {
        state.players.delete_user(username);
    }
}

fn add_user(username: &str) {
    let mut state = get_state();

    let is_self = username == state.self_name;
    let add = !state.players.name_dict.contains_key(username);

    // Only add if it's not there
    if !is_self && add {
        state.players.add(username);
    }
}

fn draw_window(ui: &Ui, not_character_or_loading: bool) {
    let state = get_state();
    if !not_character_or_loading {
        // Don't draw anything on character screen or loading screen
        return
    }

    if !state.extras_initialized {
        arcdps::imgui::Window::new("Player List Error").build(ui, || {
            ui.text("Unofficial extras extension required")
        });

        return
    };

    let mut opened_window = state.display_window;
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
            if let Some(table) = ui.begin_table_header("PLayerListTable", column_data) {
                let mut state = get_state();
                for (i, player) in state.players.player_list.iter_mut().enumerate() {
                    ui.table_next_column();
                    ui.text(&player.name);
                    ui.table_next_column();
                    ui.input_text_multiline(format!("##{i}"), &mut player.comment, [80.0, 40.0]).build();
                }
                table.end()
            };
        });
    }

    get_state().display_window = opened_window;
}

fn options(ui: &Ui, window_name: Option<&str>) -> bool {
    if let Some("error") = window_name {
        ui.checkbox("player list", &mut get_state().display_window);
    }

    false
}

fn log(msg: &str) {
    writeln!(File::options().append(true).open(TMP_PATH).unwrap(), "{msg}").unwrap();
}
