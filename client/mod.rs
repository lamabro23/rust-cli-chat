use std::env;
use std::io::{self, stdin, BufRead, Cursor};

use dotenv::dotenv;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;

fn request_username<R: BufRead>(reader: &mut R) -> io::Result<String> {
    println!("Please enter your username.");

    let mut username = String::new();
    reader.read_line(&mut username)?;
    let username = username.trim().to_string();
    match username.is_empty() {
        true => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Username cannot be empty.",
        )),
        false => Ok(username),
    }
}

async fn send_message<W>(writer: &mut W, message: String) -> io::Result<()>
where
    W: AsyncWrite + Unpin,
{
    let mut buffer = Cursor::new(message);
    writer.write_all_buf(&mut buffer).await?;
    writer.flush().await?;
    Ok(())
}

async fn send_user_messages(mut writer: OwnedWriteHalf) -> io::Result<()> {
    loop {
        let mut buff = String::new();
        stdin()
            .read_line(&mut buff)
            .expect("Reading from stdin failed!");
        send_message(&mut writer, buff).await?;
    }
}

async fn receive_server_messages(mut reader: OwnedReadHalf) -> io::Result<()> {
    loop {
        let mut buffer = vec![0; 1024];
        match reader.read(&mut buffer).await {
            Ok(n) if n > 0 => {
                let msg = String::from_utf8_lossy(&buffer).to_string();
                println!("{}", msg);
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => (),
            Err(_) | Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::ConnectionAborted,
                    "Connection with server was severed.",
                ));
            }
            Ok(_) => (),
        }
    }
}

#[tokio::main]
async fn main() -> io::Result<()> {
    dotenv().ok();
    let port = env::var("SERVER_PORT").unwrap_or_else(|_| "8080".to_string());

    let stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await?;
    let (reader, mut writer) = stream.into_split();

    let username = match request_username(&mut io::stdin().lock()) {
        Ok(result) => result,
        Err(e) => {
            eprintln!("{}", e);
            return Ok(());
        }
    };

    send_message(&mut writer, username).await?;

    tokio::spawn(async move {
        match receive_server_messages(reader).await {
            Ok(_) => (),
            Err(e) => eprintln!("{}", e),
        }
    });

    send_user_messages(writer).await?;

    Ok(())
}

#[cfg(test)]
mod client_tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_request_username_success() {
        let input = b"mytestusername\n";
        let mut reader = Cursor::new(input);

        let username = request_username(&mut reader);

        assert!(username.is_ok());
        assert_eq!(username.unwrap(), "mytestusername");
    }

    #[test]
    fn test_request_username_empty() {
        let input = b"\n";
        let mut reader = Cursor::new(input);

        let username = request_username(&mut reader);

        assert!(username.is_err());
    }

    #[test]
    fn test_request_username_trim() {
        let input = b"  mytestusername  \n";
        let mut reader = Cursor::new(input);

        let username = request_username(&mut reader).unwrap();

        assert_eq!(username, "mytestusername");
    }

    #[test]
    fn test_request_username_invalid_utf8() {
        let input = [0, 159, 146, 150]; // Invalid UTF-8 bytes
        let mut reader = Cursor::new(&input);

        let result = request_username(&mut reader);

        assert!(result.is_err());
    }
}
