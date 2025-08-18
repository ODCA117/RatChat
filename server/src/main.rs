use common::error::RatError;
use tokio::{net::{TcpListener, TcpStream}};


#[tokio::main]
async fn main() {
    println!("Starting server...");
    let listener = TcpListener::bind("127.0.0.1:6789").await.unwrap();

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            _ = connect_chat_client(socket).await;
        });
    }
}

async fn connect_chat_client(socket: TcpStream) -> Result<(), RatError> {
    let mut buf = [0;4096];
    socket.readable().await.map_err(|_| RatError::Error("Not readable".to_string()))?;
    match socket.try_read(&mut buf) {
        Ok(0) => return Ok(()),
        Ok(n) => {
            let string = String::from_utf8_lossy(&buf[..n]);
            println!("Buffer?: {:?}", string);
        },
        Err(ref e) => (),
        Err(_) => return Err(RatError::Error("Failed to read".to_string())),
    }

    Ok(())
}




