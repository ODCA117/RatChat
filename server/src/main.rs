mod chat;
mod client;

use std::{sync::Arc};
use common::{
    Id,
    error::RatError,
    message::{ChatMessage, Packet, ServerWelcomeData},
};
use log::{debug, error, info};
use serde_json;
use tokio::{
    io::AsyncReadExt, net::{TcpListener, TcpStream}, select, sync::{
        broadcast, mpsc, Mutex
    }
};
use crate::client::{ClientConnection, ClientInfo, ClientManager, SocketReader};

struct Server {
    mpsc_rx: mpsc::Receiver<String>,
    mpsc_tx: mpsc::Sender<String>,
    bc_tx: broadcast::Sender<String>,
}

impl Server {
    fn new() -> Self {
        let (bc_tx, _) = broadcast::channel(100);
        let (mpsc_tx, mpsc_rx) = mpsc::channel(100);
        Self { bc_tx, mpsc_rx, mpsc_tx }
    }

    fn get_event_handler(self) -> (EventHandler, ClientManager) {
        let ev_hand = EventHandler { mpsc_rx: self.mpsc_rx, bc_tx: self.bc_tx.clone() };
        let client_mg = ClientManager::new(self.mpsc_tx, self.bc_tx);

        (ev_hand, client_mg)
    }
}

struct EventHandler {
    mpsc_rx: mpsc::Receiver<String>,
    bc_tx: broadcast::Sender<String>,
}

impl EventHandler {
    async fn run_event_handling(mut self) {
        loop {
            debug!("Wait for events");
            match self.mpsc_rx.recv().await {
                Some(msg) => {
                    debug!("Got event: {:?}", &msg);
                    match self.bc_tx.send(msg.clone())  {
                        Ok(_) => (),
                        Err(e) => error!("Failed to send message: {:?}, {:?}", msg, e.to_string()),
                    }
                },
                None => continue,
            }
        }
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Starting server...");
    let listener = TcpListener::bind("127.0.0.1:6789").await.unwrap();
    let server = Server::new();
    let (event_handler, client_manager) = server.get_event_handler();
    let client_manager = Arc::new(Mutex::new(client_manager));

    tokio::spawn(async move {
        event_handler.run_event_handling().await; // TODO: This needs to be split into its own thing as it cannot take the lock forever.
    });

    loop {
        debug!("Wait for connection");
        let (socket, _) = listener.accept().await.unwrap();
        let cm = client_manager.clone();
        tokio::spawn(async move {
            info!("New connection arrived");
            _ = on_new_connection(socket, cm.clone()).await;
        });
    }
}

async fn on_new_connection(
    mut socket: TcpStream,
    client_manager: Arc<Mutex<ClientManager>>,
) -> Result<(), RatError> {
    let mut buf = [0; 4096];
    match socket.read(&mut buf).await {
        Ok(0) => return Ok(()), /* EOS */
        Ok(n) => {
            let mut client_manager = client_manager.lock().await;
            let mut client_con = client_manager.create_client_con(socket, &buf[..n])?;
            drop(client_manager);
            debug!("New client: {:?}", client_con);

            let reply = serde_json::to_vec(&Packet::Welcome(ServerWelcomeData { client_id: client_con.get_id() }))
                .map_err(|_| RatError::ParseError)?;
            client_con
                .write_socket(&reply)
                .await
                .map_err(|e| RatError::Error(e.to_string()))?;
            run_client_con_handler(client_con).await;
        }
        Err(e) => return Err(RatError::Error(e.to_string()))?,
    }

    Ok(())
}

async fn run_client_con_handler(mut client_con: ClientConnection) {
    let mut buf = [0; 4096];
    let (mut client_msg_listener, mut on_client_msg_send, mut client_info) = client_con.split();

    loop {
        debug!("Wait for msg");
        select! {
            // TODO: Handle disconnects!!
            client_msg_len = client_msg_listener.read_socket(&mut buf) => {
                match client_msg_len {
                    Ok(n) => {
                        match on_client_message(&client_info, &buf[..n]).await {
                            Ok(_) => (),
                            Err(e) => error!("Failed to send message as event: {:?}", e.to_string()),
                        };
                    },
                    Err(_) => error!("Failed to read socket")

                }
            }
            msg = on_client_msg_send.read_msg() => {
                let msg = msg;
                match msg {
                    Ok(msg) =>  {
                        match send_client_message(&mut client_msg_listener, &mut client_info, msg).await {
                            Ok(_) => (),
                            Err(e) => error!("Failed to send message to client: {:?}", e.to_string()),
                        };
                    },
                    Err(_) => error!("Failed to read message"),
                };
            }
        }
    }
}

async fn on_client_message<'a>(
    client_con: &ClientInfo<'a>,
    buf: &[u8],
) -> Result<(), RatError> {

    let message: Packet = serde_json::from_slice(&buf).map_err(|_| RatError::ParseError)?;
    match message {
        Packet::Message(chat_message) => {
            /* Read a msg */
            info!("Message from: {:?} : {:?}", client_con.get_name(), chat_message);
            client_con.send_msg(chat_message.message).await;
            Ok(())
        }
        Packet::Disconnect => {
            /* Graceful close down */
            Ok(())
        }
        _ => Err(RatError::ProtocolError),
    }
}

async fn send_client_message<'a>(client_socket: &mut SocketReader<'a>, client_info: &mut ClientInfo<'a>, msg: String) -> Result<(), RatError> {
    info!("client_received message: {:?}", msg);
    let data = serde_json::to_vec(&Packet::Message(ChatMessage {
        chat_id: 2,
        message: msg,
        sender_id: *client_info.get_id(),
    }))
    .map_err(|_| RatError::ParseError)?;
    client_socket
        .write_socket(&data)
        .await
        .map_err(|e| RatError::Error(e.to_string()))?;
    Ok(())
}
