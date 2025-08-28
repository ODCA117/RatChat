use std::time::Duration;
use log::{self, trace, debug, info, error};
use env_logger;
use clap::Parser;

use common::{
    error::RatError,
    message::{ChatMessage, Client, ClientConnectData, Packet},
};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

#[derive(Parser)]
struct Args {
    #[arg(short, long)]
    name: String,

    #[arg(short, long)]
    addr: String,
}

struct ServerConnection {
    stream: TcpStream,
}

#[tokio::main]
async fn main() -> Result<(), RatError> {
    env_logger::init();
    let args = Args::parse();
    debug!("Connect to server: 127.0.0.1:6789");
    let (connection, client) = connect_to_server(&args.addr, args.name).await?;
    process(client, connection).await?;
    Ok(())
}

async fn connect_to_server(addr: &str, name: String) -> Result<(ServerConnection, Client), RatError> {
    let mut stream = TcpStream::connect(addr)
        .await
        .map_err(|_| RatError::Error("Not possible to connect".to_string()))?;
    // Wait for the socket to be writable
    let hello = Packet::Connect(ClientConnectData::new(name.clone()));
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
                        Packet::Welcome(_welcome_data) => {
                            info!("Server accepted");
                            let client = Client { id: _welcome_data.client_id, name };
                            return Ok((ServerConnection { stream }, client));
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

async fn process(client: Client, mut connection: ServerConnection) -> Result<(), RatError> {
    let mut buf = [0; 4096];
    loop {
        trace!("Client waiting for action");
        tokio::select! {
            // TODO: Handle disconnects!!
            server_msg = connection.stream.read(&mut buf) => { read_msg_from_server(server_msg, &buf).await?; }
            user_input = read_from_user() => { send_msg_to_server(&client, &mut connection, user_input).await?; }
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
                Packet::Message(chat) => {
                    println!("{:?}:\n{:?}", chat.sender_id, chat.message);
                    Ok(())
                }
                Packet::Disconnect => Ok(()),
                _ => Err(RatError::ProtocolError),
            }
        }
        Err(e) => Err(RatError::Error(e.to_string())),
    }
}

async fn send_msg_to_server(
    client: &Client,
    connection: &mut ServerConnection,
    user_input: Result<String, RatError>,
) -> Result<(), RatError> {
    let Ok(input) = user_input else {
        error!("Failed to get message from user");
        return Ok(());
    };

    if input.is_empty() {
        return Ok(());
    }

    let packet = Packet::Message(ChatMessage {
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
    debug!("Message sent");
    Ok(())
}

async fn read_from_user() -> Result<String, RatError> {
    let stdin = BufReader::new(tokio::io::stdin());
    let mut lines = stdin.lines();
    if let Some(line) = lines
        .next_line()
        .await
        .map_err(|e| RatError::Error(e.to_string()))?
    {
        Ok(line)
    } else {
        Ok(String::new())
    }
}
