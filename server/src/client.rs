use std::{collections::HashMap, io};

use common::{error::RatError, message::{Client, Packet}, Id};
use log::info;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    select,
    sync::{broadcast, mpsc},
};

pub struct ClientManager {
    clients: HashMap<Id, Client>,
    id: Id,
    mpsc_tx: mpsc::Sender<String>,
    bc_tx: broadcast::Sender<String>,
}

impl ClientManager {
    pub fn new(mpsc_tx: mpsc::Sender<String>, bc_tx: broadcast::Sender<String>) -> Self {
        Self {
            clients: HashMap::new(),
            id: 0,
            mpsc_tx,
            bc_tx,
        }
    }

    pub fn create_client_con(&mut self, socket: TcpStream, buf: &[u8]) -> Result<ClientConnection, RatError> {
        let id = self.generate_client_id();
        let mpsc_tx = self.mpsc_tx.clone();
        let bc_rx = self.bc_tx.subscribe();
        let msg: Packet = serde_json::from_slice(buf).map_err(|_| RatError::ParseError)?;
        let (client, client_con) = match msg {
            Packet::Connect(client_con) => {

                (Client {id, name: client_con.get_name()}, ClientConnection::new(id, client_con.get_name(), bc_rx, mpsc_tx, socket))
            },
            _ => return Err(RatError::ProtocolError)?,
        };

        self.insert(client); // NOTE: Not sure this is needed for now
        return Ok(client_con); // This will work as long as the internals are not updated. Maybe Client is to generic for what it is, more of a ClientConnection
    }

    fn generate_client_id(&self) -> Id {
        let mut id = self.clients.len();
        loop {
            if !self.clients.contains_key(&id) {
                return id;
            }
            id += 1;
        }
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }

    pub fn insert(&mut self, client: Client) {
        self.clients.insert(client.id, client);
    }

    pub fn get_mut(&mut self, id: &Id) -> Option<&mut Client> {
        self.clients.get_mut(&id)
    }
}

pub struct SocketReader<'a> {
    socket: &'a mut TcpStream,
}

impl<'a> SocketReader<'a> {
    pub async fn read_socket(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.socket.read(buf).await
    }

    pub async fn write_socket(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.socket.write(buf).await
    }
}

pub struct BroadcastReceiver<'a> {
    bc_rx: &'a mut broadcast::Receiver<String>,
}

impl<'a> BroadcastReceiver<'a> {
    pub async fn read_msg(&mut self) -> Result<String, RatError> {
        self.bc_rx.recv().await.map_err(|e| RatError::Error(e.to_string()))
    }
}

pub struct ClientInfo<'a> {
    id: &'a Id,
    name: &'a String,
    mpsc_tx: &'a mpsc::Sender<String>,
}

impl<'a> ClientInfo<'a> {
    pub fn get_id(&self) -> &Id {
        self.id
    }

    pub fn get_name(&self) -> &str {
        self.name
    }

    pub async fn send_msg(&self, msg: String) -> Result<(), RatError> {
        info!("send message {:?} on {:?}", &msg,  &self.mpsc_tx);
        self.mpsc_tx
            .send(msg)
            .await
            .map_err(|e| RatError::Error(e.to_string()))?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ClientConnection {
    id: Id,
    name: String,
    socket: TcpStream,
    bc_rx: broadcast::Receiver<String>, // Receives data on this one
    mpsc_tx: mpsc::Sender<String>,      // Sends data to this one
}

impl ClientConnection {
    pub fn new(
        id: Id,
        name: String,
        bc_rx: broadcast::Receiver<String>,
        mpsc_tx: mpsc::Sender<String>,
        socket: TcpStream,
    ) -> Self {
        ClientConnection {
            id,
            name,
            bc_rx,
            mpsc_tx,
            socket,
        }
    }

    pub fn get_id(&self) -> Id {
        self.id
    }

    pub fn get_name(&self) -> String {
        self.name.clone()
    }

    pub fn split(&mut self) -> (SocketReader, BroadcastReceiver, ClientInfo) {
        (
            SocketReader { socket: &mut self.socket },
            BroadcastReceiver { bc_rx: &mut self.bc_rx },
            ClientInfo { id: &mut self.id, name: &mut self.name, mpsc_tx: &mut self.mpsc_tx }
        )
    }

    pub async fn send_msg(&self, msg: String) -> Result<(), RatError> {
        self.mpsc_tx
            .send(msg)
            .await
            .map_err(|e| RatError::Error(e.to_string()))?;
        Ok(())
    }

    pub async fn recv_msg(&mut self) -> Result<String, RatError> {
        Ok(self
            .bc_rx
            .recv()
            .await
            .map_err(|e| RatError::Error(e.to_string()))?)
    }

    pub async fn read_socket(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.socket.read(buf).await
    }

    pub async fn write_socket(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.socket.write(buf).await
    }
}
