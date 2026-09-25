use std::collections::HashMap;

use toml::{Value, map::Map};

use crate::{Filters, Flags, Player, PlayerVecMap, State};

pub mod v1;

const CONFIG_PATH: &'static str = "addons/arcdps/player_list.toml";
#[cfg(debug_assertions)]
pub const TMP_PATH: &'static str = "addons/arcdps/player_list.tmp";
const CURRENT_CONFIG_VERSION: i64 = 1;

const PLAYERS: &'static str = "Players";
const OPENED_WINDOW: &'static str = "WindowOpen";
const INACTIVE_COLOR: &'static str = "InactiveColor";
const SHOW_ALL: &'static str = "ShowAll";
const COMMENT_SIZE: &'static str = "CommentSize";
const AUTO_CHECK_UPDATE: &'static str = "AutoCheckUpdate";
const AUTO_CHECK_BETA: &'static str = "AutoCheckBeta";
const CONFIG_VERSION: &'static str = "ConfigVersion";
const DEFAULT_INACTIVE_COLOR: [f32;4] = [0.5,0.5,0.5,1.0];
const DEFAULT_COMMENT_SIZE: [f32;2] = [300.0, 20.0];
const DEFAULT_AUTO_CHECK_UPDATE: bool = true;
const DEFAULT_AUTO_CHECK_BETA: bool = false;
const SHORTCUT: &'static str = "ShortcutKey";

pub fn parse(state: &mut State) -> Result<(), Option<String>> {
    let toml_string = std::fs::read_to_string(CONFIG_PATH).unwrap_or_default();
    let mut config = match toml::from_str::<Value>(&toml_string)
        .unwrap_or(Value::Table(Map::new())) {
            Value::Table(config) => config,
            _ => Map::new()
        };


    let version = match config.remove(CONFIG_VERSION) {
        Some(Value::Integer(i)) => i,
        None => 1, // pre-release player_list did not have config version. pre-release config is compatible with version 1, so assume it is version 1
        _ => return Err(Some("Database version not supported".to_string())),
    };

    match version {
        1 => v1::parse_config(state, config),
        _ => return Err(Some("Database version not supported".to_string())),
    }

    Ok(())
}

pub fn save(state: &mut State) -> Result<(), String> {
    let mut config = Map::new();

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
    config.insert(AUTO_CHECK_UPDATE.to_string(), Value::Boolean(state.auto_check_update));
    config.insert(AUTO_CHECK_BETA.to_string(), Value::Boolean(state.auto_check_beta));
    config.insert(CONFIG_VERSION.to_string(), Value::Integer(CURRENT_CONFIG_VERSION));

    let Ok(toml_string) = toml::to_string(&Value::Table(config)) else {
        return Err("Could not serialize configuration".to_string())
    };
    if let Err(_) = std::fs::write(CONFIG_PATH, toml_string) {
        return Err("Could not save configuration".to_string())
    }

    Ok(())
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

pub fn default_state() -> State {
    State {
        players: PlayerVecMap::new(),
        self_name: "".to_string(),
        flags: Flags::new(),
        filters: Filters::new(),
        inactive_color: DEFAULT_INACTIVE_COLOR,
        comment_size: DEFAULT_COMMENT_SIZE,
        add_user_text: "".to_string(),
        shortcut_char: None,
        listening_to_key: false,
        auto_check_update: DEFAULT_AUTO_CHECK_UPDATE,
        auto_check_beta: DEFAULT_AUTO_CHECK_BETA,
    }
}
