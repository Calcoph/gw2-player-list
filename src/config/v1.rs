use toml::{Value, map::Map};
use windows::System::VirtualKey;

use crate::{State, config::{AUTO_CHECK_BETA, AUTO_CHECK_UPDATE, COMMENT_SIZE, DEFAULT_AUTO_CHECK_BETA, DEFAULT_AUTO_CHECK_UPDATE, DEFAULT_COMMENT_SIZE, DEFAULT_INACTIVE_COLOR, INACTIVE_COLOR, OPENED_WINDOW, SHORTCUT, SHOW_ALL, init_player_list}};

pub fn parse_config(state: &mut State, mut config: Map<String, Value>) {
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
    let auto_check_update = match config.remove(AUTO_CHECK_UPDATE) {
        Some(Value::Boolean(b)) => b,
        _ => DEFAULT_AUTO_CHECK_UPDATE,
    };
    let auto_check_beta = match config.remove(AUTO_CHECK_BETA) {
        Some(Value::Boolean(b)) => b,
        _ => DEFAULT_AUTO_CHECK_BETA,
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

    state.players = player_list;
    state.flags.display_window = display_window;
    state.flags.show_all = show_all;
    state.inactive_color = inactive_color;
    state.comment_size = comment_size;
    state.shortcut_char = shortcut_char;
    state.auto_check_update = auto_check_update;
    state.auto_check_beta = auto_check_beta;
}
