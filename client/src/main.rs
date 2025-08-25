use std::time::Duration;
use log::{self, debug, trace};
use env_logger;

use common::{
    error::RatError,
    message::{ChatMessage, Client, Packet},
};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

struct ServerConnection {
    stream: TcpStream,
}

#[tokio::main]
async fn main() -> Result<(), RatError> {
    env_logger::init();
    debug!("Connect to server: 127.0.0.1:6789");
    let connection = connect_to_server("127.0.0.1:6789").await?;
    process(connection).await?;
    Ok(())
}

async fn connect_to_server(addr: &str) -> Result<ServerConnection, RatError> {
    let mut stream = TcpStream::connect(addr)
        .await
        .map_err(|_| RatError::Error("Not possible to connect".to_string()))?;
    // Wait for the socket to be writable
    let hello = Packet::ClientHello(Client {
        id: 1,
        name: "Calle".to_string(),
    });
    let data = serde_json::to_vec(&hello).map_err(|_| RatError::ParseError)?;

    // Try to write data, this may still fail with `WouldBlock`
    // if the readiness event is a false positive.
    debug!("Send hello");
    match stream.write(&data).await {
        Ok(_) => {
            /* Was able to send hello to server. Waiting for acceptance */
            let mut buf = [0; 4096];
            match stream.read(&mut buf).await {
                Ok(0) => return Err(RatError::Error("Connection closed early".to_string())),
                Ok(n) => {
                    /* Got reply from server */
                    let data: Packet =
                        serde_json::from_slice(&buf[..n]).map_err(|_| RatError::ParseError)?;
                    match data {
                        Packet::ServerAccept => {
                            println!("Server accepted");
                            return Ok(ServerConnection { stream });
                        }
                        _ => return Err(RatError::ProtocolError),
                    }
                }
                Err(_) => return Err(RatError::Error("Handshake failed".to_string())),
            }
        }
        Err(e) => return Err(RatError::Error(e.to_string())),
    }
}

async fn process(mut connection: ServerConnection) -> Result<(), RatError> {
    let mut buf = [0; 4096];
    loop {
        trace!("Client waiting for action");
        tokio::select! {
            // TODO: Handle disconnects!!
            server_msg = connection.stream.read(&mut buf) => { read_msg_from_server(server_msg, &buf).await?; }
            user_input = read_from_user() => { send_msg_to_server(&mut connection, user_input).await?; }
        }
    }
}

async fn read_msg_from_server(read_res: io::Result<usize>, buf: &[u8]) -> Result<(), RatError> {
    match read_res {
        Ok(0) => Ok(()),
        Ok(n) => {
            let packet: Packet =
                serde_json::from_slice(&buf[..n]).map_err(|_| RatError::ProtocolError)?;
            match packet {
                Packet::Chat(chat) => {
                    println!("New message:\n{:?}", chat);
                    Ok(())
                }
                Packet::Bye => Ok(()),
                _ => Err(RatError::ProtocolError),
            }
        }
        Err(e) => Err(RatError::Error(e.to_string())),
    }
}

async fn send_msg_to_server(
    connection: &mut ServerConnection,
    user_input: Result<String, RatError>,
) -> Result<(), RatError> {
    let Ok(input) = user_input else {
        println!("Failed to get message from user");
        return Ok(());
    };

    if input.is_empty() {
        println!("Empty string");
        return Ok(());
    }

    let packet = Packet::Chat(ChatMessage {
        chat_id: 1,
        sender_id: 1,
        message: input,
    });
    let data = serde_json::to_vec(&packet).map_err(|_| RatError::ParseError)?;
    connection
        .stream
        .write(&data)
        .await
        .map_err(|e| RatError::Error(e.to_string()))?;
    println!("Message sent");
    Ok(())
}

async fn read_from_user() -> Result<String, RatError> {
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    println!("Wait for line from stdin");
    if let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| RatError::Error(e.to_string()))?
    {
        println!("Got a line:{line}");
        Ok(line)
    } else {
        Ok(String::new())
    }
}
