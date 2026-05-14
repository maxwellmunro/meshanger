use bincode::{Decode, Encode};

use crate::server::data::UserSet;

#[derive(Clone, Encode, Decode, Debug)]
pub enum Packet {
    /* S <--- C */ JoinRequest { username: String },
    /* S ---> C */ JoinResponseSuccess { id: u64, counter: u64, users: UserSet },
    /* S ---> C */ JoinResponseDeny { err: String },
    /* S ---> C */ UserJoined { username: String },

    /* S <--- C */ LeaveRequest,
    /* S ---> C */ UserLeft { id: u64 },
    /* S ---> C */ Kick { reason: String },
    /* S <--> S */ InternalClientDisconnect,

    /* S <--> C */ Chat { counter: u64, sender_id: u64, ciphertext: Vec<u8> },
}
