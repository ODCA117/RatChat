use serde::{Serialize, Deserialize};

use crate::Id;

#[derive(Serialize, Deserialize, Debug)]
pub enum Packet {
    ClientHello(Client), // Should probably contain some form of identification.
    ServerAccept,
    Chat(ChatMessage), // Is the actual message
    Bye,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Client {
    pub id: Id,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMessage {
    pub chat_id: Id,
    pub sender_id: Id,
    pub message: String,
    // Can add other meta data such as sent, recieved, time etc...
}
