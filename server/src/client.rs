use std::collections::HashMap;

use common::{error::RatError, Id};
use tokio::sync::broadcast::{Receiver, Sender};

pub struct ClientManager {
    clients: HashMap<Id, Client>
}

impl ClientManager {
    pub fn new() -> Self {
        Self { clients: HashMap::new() }
    }
}

#[derive(Debug)]
pub struct Client {
    id: Id,
    name: String,
    connected: bool,
    rx: Receiver<String>,
    tx: Sender<String>,
}

impl Client {
    pub fn new(id: Id, name: String, rx: Receiver<String>, tx: Sender<String>) -> Self {
        Client { id, name, connected: true, rx, tx }
    }

    pub fn send_msg(&self, msg: String) -> Result<(), RatError>{
        self.tx.send(msg).map_err(|e| RatError::Error(e.to_string()))?;
        Ok(())
    }

    pub async fn recv_msg(&mut self) -> Result<String, RatError> {
        Ok(self.rx.recv().await.map_err(|e| RatError::Error(e.to_string()))?)
    }
}
