use std::collections::HashMap;

use toml::{map::Map, Value};

pub struct Player {
    pub name: String,
    pub lowercase_name: String,
    pub comment: String,
    pub lowercase_comment: String,
    pub in_squad: bool
}

impl Player {
    pub fn to_toml(&self) -> Value {
        let mut toml_map = Map::new();

        toml_map.insert("name".to_string(), Value::String(self.name.clone()));
        toml_map.insert("comment".to_string(), Value::String(self.comment.clone()));

        Value::Table(toml_map)
    }
}

pub struct PlayerVecMap {
    pub player_list: Vec<Player>,
    pub name_dict: HashMap<String, usize>
}

impl PlayerVecMap {
    pub fn new() -> PlayerVecMap {
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

    pub fn user_left(&mut self, username: &str) {
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
    pub fn delete(&mut self, username: &str) {
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
    pub fn delete_all(&mut self) {
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

    pub fn join(&mut self, username: &str) {
        self.add_player(username, "".to_string());

        if let Some(index) = self.name_dict.get(username) {
            let player = &mut self.player_list[*index];
            player.in_squad = true;
        };
    }

    pub fn add_player(&mut self, username: &str, comment: String) {
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
