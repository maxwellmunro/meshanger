use std::sync::Arc;

use chacha20poly1305::XChaCha20Poly1305;
use tokio::sync::RwLock;

use crate::server::data::UserSet;

#[derive(Clone)]
pub struct Data {
    pub id: Arc<RwLock<u64>>,
    pub counter: Arc<RwLock<u64>>,
    pub cipher: Arc<RwLock<Option<XChaCha20Poly1305>>>,
    pub users: Arc<RwLock<UserSet>>,
}

impl Data {
    pub fn new() -> Data {
        Data {
            id: Arc::new(RwLock::new(0)),
            counter: Arc::new(RwLock::new(0)),
            cipher: Arc::new(RwLock::new(None)),
            users: Arc::new(RwLock::new(UserSet::new())),
        }
    }
}
