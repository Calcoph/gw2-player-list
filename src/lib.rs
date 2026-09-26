#![allow(static_mut_refs)]
const VERSION_MAJOR: u32 = 1;
const VERSION_MINOR: u32 = 0;
const VERSION_PATCH: u32 = 0;

use std::sync::{Mutex, MutexGuard};
use arcdps::{extras::{ExtrasAddonInfo, UserInfoIter}, imgui::{Ui}};
use once_cell::sync::Lazy;
use windows::System::VirtualKey;

use crate::{config::{Config, UpdaterData}, player_vec_map::PlayerVecMap};

mod config;
mod gui;
mod player_vec_map;
mod update_checker;

arcdps::export! {
    name: "Player List",
    sig: 0x73242FB, // random number
    init,
    extras_init: init_extras,
    release,
    imgui: draw_window,
    extras_squad_update: squad_update,
    options_windows: arcdps_options_hook,
    options_end: addon_options,
    wnd_filter: shortcuts,
    wnd_nofilter: nofilter,
    //update_url: automatic_update_checker, Does not work with unofficial extras.
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
    ignore_updater_window: bool,
    show_all: bool,
    listening_to_key: bool,
    config_correctly_parsed: bool,
}

impl Flags {
    fn new() -> Flags {
        Flags {
            extras_initialized: false,
            display_window: false,
            ignore_updater_window: false,
            show_all: false,
            listening_to_key: false,
            config_correctly_parsed: false,
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
    updater_data: UpdaterData,
}

static mut STATE: Lazy<Mutex<State>> = Lazy::new(|| Mutex::new(config::default_state()));


// automatic_update_checker may be called before init() is called. Be careful
fn init() -> Result<(), Option<String>> {
    // May return an error to indicate load failure

    let mut state = get_state();
    read_config(&mut state)?;
    update_checker::after_update_cleanup();
    if state.updater_data.available_version.is_none() {
        update_checker::check_for_updates(&mut state);
    }

    #[cfg(debug_assertions)] // In order to work with arcdps_mock
    {
        if !state.flags.extras_initialized {
            extras_initializer(state, Some("abcdtest"));
        }
    }

    log("Initialized");
    Ok(())
}

fn read_config(state: &mut State) -> Result<(), Option<String>> {
    if state.flags.config_correctly_parsed {
        return Ok(())
    }

    config::parse(state)?;
    state.flags.config_correctly_parsed = true;

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
    log("Extras initialized");
    let state = get_state();
    extras_initializer(state, self_name);
}

fn release() {
    let mut state = get_state();
    save_state(&mut state);
}

fn save_state(state: &mut State) {
    if state.flags.config_correctly_parsed { // Only save if we actually read the config file, in order to not overwrite data
        if let Err(_) = config::save(state) {
            log("Failed to save!")
        };
        log("Saved")
    } else {
        log("Not saving config since could not correctly parse in the first place")
    }
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
    let mut state = get_state();
    if !not_character_or_loading {
        // Don't draw anything on character screen or loading screen
        return
    }

    gui::main_window(ui, &mut state)
}

enum Action {
    DeletePlayer(String)
}

fn arcdps_options_hook(ui: &Ui, window_name: Option<&str>) -> bool {
    let mut state = get_state();

    if let Some("error") = window_name {
        gui::arcdps_options(ui, &mut state)
    }

    false
}

fn addon_options(ui: &Ui) {
    let mut state = get_state();
    gui::options(ui, &mut state);
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
