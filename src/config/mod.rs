use std::collections::HashMap;

use toml::{Value, map::Map};
use windows::System::VirtualKey;

use crate::{Filters, Flags, PlayerVecMap, State, player_vec_map::Player};

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
const UPDATER_DATA: &'static str = "UpdaterData";
const DEFAULT_INACTIVE_COLOR: [f32;4] = [0.5,0.5,0.5,1.0];
const DEFAULT_COMMENT_SIZE: [f32;2] = [300.0, 20.0];
const DEFAULT_SHORTCUT_CHAR: Option<VirtualKey> = None;
const DEFAULT_AUTO_CHECK_UPDATE: bool = true;
const DEFAULT_AUTO_CHECK_BETA: bool = false;
const DEFAULT_DAYS_BETWEEN_POLLS: u64 = 30;
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
        1 => v1::parse_config(state, config)?,
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
    let inactive_color = state.config.inactive_color.into_iter()
        .map(|val| Value::Float(val as f64)).collect();
    config.insert(INACTIVE_COLOR.to_string(), Value::Array(inactive_color));
    let comment_size = state.config.comment_size.into_iter()
        .map(|val| Value::Float(val as f64)).collect();
    config.insert(COMMENT_SIZE.to_string(), Value::Array(comment_size));
    config.insert(SHOW_ALL.to_string(), Value::Boolean(state.flags.show_all));
    if let Some(i) = state.config.shortcut_char {
        config.insert(SHORTCUT.to_string(), Value::Integer(i.0 as i64));
    }
    config.insert(AUTO_CHECK_UPDATE.to_string(), Value::Boolean(state.config.auto_check_update));
    config.insert(AUTO_CHECK_BETA.to_string(), Value::Boolean(state.config.auto_check_beta));
    config.insert(CONFIG_VERSION.to_string(), Value::Integer(CURRENT_CONFIG_VERSION));

    config.insert(UPDATER_DATA.to_string(), state.updater_data.to_value());

    let Ok(toml_string) = toml::to_string(&Value::Table(config)) else {
        return Err("Could not serialize configuration".to_string())
    };
    if let Err(_) = std::fs::write(CONFIG_PATH, toml_string) {
        return Err("Could not save configuration".to_string())
    }

    Ok(())
}

fn init_player_list(config: &mut Map<String, Value>) -> Result<PlayerVecMap, String> {
    let players = config.remove(PLAYERS);

    let players = match players {
        Some(Value::Array(players)) => players,
        None => vec![],
        _ => return Err("Could not parse player notes".to_string()),
    };

    let mut player_map = HashMap::new();

    let mut error = false;
    let player_list: Vec<_> = players.into_iter()
        .filter_map(|val| {
            let mut properties = match val {
                Value::Table(properties) => properties,
                _ => {
                    error = true;
                    return None
                }
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
                error = true;
                None
            }
        }).collect();

    if error {
        return Err("Could not parse player notes".to_string());
    }

    for (i, player) in player_list.iter().enumerate() {
        player_map.insert(player.name.clone(), i);
    }

    Ok(PlayerVecMap {
        player_list,
        name_dict: player_map,
    })
}

pub fn default_state() -> State {
    State {
        players: PlayerVecMap::new(),
        self_name: "".to_string(),
        flags: Flags::new(),
        filters: Filters::new(),
        add_user_text: "".to_string(),
        config: Config {
            inactive_color: DEFAULT_INACTIVE_COLOR,
            comment_size: DEFAULT_COMMENT_SIZE,
            shortcut_char: DEFAULT_SHORTCUT_CHAR,
            auto_check_update: DEFAULT_AUTO_CHECK_UPDATE,
            auto_check_beta: DEFAULT_AUTO_CHECK_BETA,
        },
        updater_data: UpdaterData::new(),
    }
}

pub struct Config {
    pub inactive_color: [f32;4],
    pub comment_size: [f32;2],
    pub shortcut_char: Option<VirtualKey>,
    pub auto_check_update: bool,
    pub auto_check_beta: bool,
}

const MAJOR: &'static str = "Major";
const MINOR: &'static str = "Minor";
const PATCH: &'static str = "Patch";
const URL: &'static str = "Url";

#[derive(Debug, Clone)]
pub struct AvailableVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub url: String,
}
impl AvailableVersion {
    fn to_value(&self) -> Value {
        let mut value = Map::new();

        value.insert(MAJOR.to_string(), Value::Integer(self.major as i64));
        value.insert(MINOR.to_string(), Value::Integer(self.minor as i64));
        value.insert(PATCH.to_string(), Value::Integer(self.patch as i64));
        value.insert(URL.to_string(), Value::String(self.url.clone()));

        Value::Table(value)
    }

    fn v1_parse(mut version: Map<String, Value>) -> Option<AvailableVersion> {
        let major = match version.remove(MAJOR) {
            Some(Value::Integer(i)) => i as u32,
            _ => return None,
        };
        let minor = match version.remove(MINOR) {
            Some(Value::Integer(i)) => i as u32,
            _ => return None,
        };
        let patch = match version.remove(PATCH) {
            Some(Value::Integer(i)) => i as u32,
            _ => return None,
        };
        let url = match version.remove(URL) {
            Some(Value::String(s)) => s,
            _ => return None,
        };

        Some(AvailableVersion { major, minor, patch, url })
    }
}

const LAST_UPDATE_TIMESTAMP: &'static str = "LastUpdateTimestamp";
const DAYS_BETWEEN_POLLS: &'static str = "DaysBetweenPolls";
const ETAG: &'static str = "Etag";
const LAST_MODIFIED: &'static str = "LastModified";
const AVAILABLE_VERSION: &'static str = "AvailableVersion";

pub struct UpdaterData {
    pub last_update_timestamp: u64,
    pub days_between_polls: u64,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
    pub available_version: Option<AvailableVersion>, // TODO: Do something with this
}
impl UpdaterData {
    fn new() -> Self {
        Self {
            last_update_timestamp: 0,
            days_between_polls: DEFAULT_DAYS_BETWEEN_POLLS,
            etag: None,
            last_modified: None,
            available_version: None
        }
    }

    fn to_value(&self) -> Value {
        let mut value = Map::new();

        value.insert(LAST_UPDATE_TIMESTAMP.to_string(), Value::Integer(self.last_update_timestamp as i64));
        value.insert(DAYS_BETWEEN_POLLS.to_string(), Value::Integer(self.days_between_polls as i64));
        if let Some(etag) = self.etag.as_ref() {
            value.insert(ETAG.to_string(), Value::String(etag.clone()));
        }
        if let Some(last_modified) = self.last_modified.as_ref() {
            value.insert(LAST_MODIFIED.to_string(), Value::String(last_modified.clone()));
        }
        if let Some(available_version) = self.available_version.as_ref() {
            value.insert(AVAILABLE_VERSION.to_string(), available_version.to_value());
        }

        Value::Table(value)
    }

    fn v1_parse(mut updater_data: Map<String, Value>) -> UpdaterData {
        let last_update_timestamp = match updater_data.remove(LAST_UPDATE_TIMESTAMP) {
           Some(Value::Integer(i)) => i as u64,
           _ => 0,
        };
        let days_between_polls = match updater_data.remove(DAYS_BETWEEN_POLLS) {
            Some(Value::Integer(i)) => i as u64,
            _ => DEFAULT_DAYS_BETWEEN_POLLS,
        };
        let etag = match updater_data.remove(ETAG) {
            Some(Value::String(s)) => Some(s),
            _ => None,
        };
        let last_modified = match updater_data.remove(LAST_MODIFIED) {
            Some(Value::String(s)) => Some(s),
            _ => None,
        };
        let available_version = match updater_data.remove(AVAILABLE_VERSION) {
            Some(Value::Table(version)) => AvailableVersion::v1_parse(version),
            _ => None,
        };

        UpdaterData {
            last_update_timestamp,
            days_between_polls,
            etag,
            last_modified,
            available_version,
        }
    }
}
