use std::{
    collections::{
        HashMap,
        hash_map::{Iter, IterMut},
    },
    sync::Arc,
};

use bincode::{Decode, Encode};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct Data {
    pub users: Arc<RwLock<UserSet>>,
    pub counter: Arc<RwLock<u64>>,
}

impl Data {
    pub fn new() -> Data {
        Data {
            users: Arc::new(RwLock::new(UserSet::new())),
            counter: Arc::new(RwLock::new(0)),
        }
    }
}

#[derive(Clone, Encode, Decode, Debug)]
pub struct UserSet {
    users: HashMap<u64, String>,
    next_id: u64,
}

impl UserSet {
    pub fn new() -> UserSet {
        UserSet {
            users: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn insert(&mut self, username: String) -> u64 {
        let id = self.next_id;
        self.users.insert(id, username);
        self.next_id += 1;
        id
    }

    pub fn remove(&mut self, id: &u64) -> Option<String> {
        self.users.remove(id)
    }

    pub fn get(&self, id: &u64) -> Option<&String> {
        self.users.get(id)
    }

    pub fn get_mut(&mut self, id: &u64) -> Option<&mut String> {
        self.users.get_mut(id)
    }

    pub fn iter(&self) -> Iter<'_, u64, String> {
        self.users.iter()
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, u64, String> {
        self.users.iter_mut()
    }
}
