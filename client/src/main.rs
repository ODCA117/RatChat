use common::error::RatError;
use tokio::{io, net::TcpStream};

#[tokio::main]
async fn main() -> Result<(), RatError> {
    let stream = TcpStream::connect("127.0.0.1:6789").await.map_err(|_| RatError::Error("Not possible to connect".to_string()))?;
    loop {
        // Wait for the socket to be writable
        stream.writable().await.map_err(|_| RatError::Error("Not writable".to_string()))?;

        // Try to write data, this may still fail with `WouldBlock`
        // if the readiness event is a false positive.
        match stream.try_write(b"hello world") {
            Ok(n) => {
                break;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                return Err(RatError::Error("Failed".to_string()));
            }
        }
    }
    Ok(())
}
