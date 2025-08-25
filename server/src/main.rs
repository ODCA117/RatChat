mod chat;
mod client;

use std::time::Duration;

use common::{
    error::RatError,
    message::{ChatMessage, Packet},
};
use serde_json;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::broadcast::{self, Receiver, Sender},
};

use crate::client::{Client, ClientManager};

struct Server {
    client_manager: ClientManager,
}

#[tokio::main]
async fn main() {
    println!("Starting server...");
    let listener = TcpListener::bind("127.0.0.1:6789").await.unwrap();
    let (tx, _) = broadcast::channel(100);

    // let server = Server { client_manager: ClientManager::new() };

    loop {
        println!("Listen on incomming connections");
        let (socket, _) = listener.accept().await.unwrap();
        println!("Connection accepted");
        let tx = tx.clone();
        tokio::spawn(async move {
            _ = on_incoming_connection(socket, tx).await;
        });
    }
}

async fn on_incoming_connection(mut socket: TcpStream, tx: Sender<String>) -> Result<(), RatError> {
    let rx = tx.subscribe();
    let mut buf = [0; 4096];
    match socket.read(&mut buf).await {
        Ok(0) => return Ok(()), /* EOS */
        Ok(n) => {
            let client = get_client(&buf[..n], rx, tx)?;

            let reply =
                serde_json::to_vec(&Packet::ServerAccept).map_err(|_| RatError::ParseError)?;
            socket
                .write(&reply)
                .await
                .map_err(|e| RatError::Error(e.to_string()))?;
            client_chat(client, &mut socket).await?;
        }
        Err(e) => return Err(RatError::Error(e.to_string()))?,
    }

    Ok(())
}

fn get_client(buf: &[u8], rx: Receiver<String>, tx: Sender<String>) -> Result<client::Client, RatError> {
    let msg: Packet = serde_json::from_slice(buf).map_err(|_| RatError::ParseError)?;
    match msg {
        Packet::ClientHello(client) => return Ok(Client::new(client.id, client.name, rx, tx)),
        _ => return Err(RatError::ProtocolError),
    }
}

async fn client_chat(mut client: Client, socket: &mut TcpStream) -> Result<(), RatError> {
    let mut buf = [0; 4096];
    loop {
        println!("Server is waiting for action");
        tokio::select! {
            // TODO: Handle disconnects!!
            client_msg = socket.read(&mut buf) => {client_sent_msg(&client, client_msg, &buf).await?; }
            msg = client.recv_msg() => { client_recv_msg(socket, msg.unwrap()).await?;}
        }
    }
}

async fn client_sent_msg(client: &Client, client_msg: std::io::Result<usize>, buf: &[u8]) -> Result<(), RatError> {
    let Ok(n) = client_msg else {
        return Err(RatError::Error(
            "Failed to get message from client".to_string(),
        ));
    };
    if n == 0 {
        return Ok(());
    }

    let message: Packet = serde_json::from_slice(&buf[..n]).map_err(|_| RatError::ParseError)?;
    match message {
        Packet::Chat(chat_message) => {
            /* Read a msg */
            println!("Forward Message: {:?}", chat_message);
            client.send_msg(chat_message.message);
            Ok(())
        }
        Packet::Bye => {
            /* Graceful close down */
            Ok(())
        }
        _ => Err(RatError::ProtocolError),
    }
}

async fn client_recv_msg(stream: &mut TcpStream, msg: String) -> Result<(), RatError> {
    println!("client_received message: {:?}", msg);
    let data = serde_json::to_vec(&Packet::Chat(ChatMessage {
        chat_id: 2,
        message: msg,
        sender_id: 21,
    }))
    .map_err(|_| RatError::ParseError)?;
    stream
        .write(&data)
        .await
        .map_err(|e| RatError::Error(e.to_string()))?;
    Ok(())
}
