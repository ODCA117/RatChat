use serde::{Deserialize, Serialize};

use crate::Id;

#[derive(Serialize, Deserialize, Debug)]
pub enum Packet {
    Connect(ClientConnectData), // Should probably contain some form of identification.
    Welcome(ServerWelcomeData),
    Message(ChatMessage), // Is the actual message
    Disconnect,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClientConnectData {
    name: String,
}

impl ClientConnectData {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServerWelcomeData {
    pub client_id: Id,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMessage {
    pub chat_id: Id,
    pub sender_id: Id,
    pub message: String,
    // Can add other meta data such as sent, recieved, time etc...
}

#[derive(Debug, Clone)]
pub struct Client {
    pub id: Id,
    pub name: String,
}
