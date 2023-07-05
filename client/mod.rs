use std::io::{self, BufRead, Cursor};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
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

async fn wait_for_message<R: BufRead>(reader: R) -> io::Result<String> {
    let mut message = String::new();
    let mut reader = reader;
    reader.read_line(&mut message)?;
    Ok(message.trim().to_string())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let username = request_username(&mut io::stdin().lock()).unwrap();
    // let username = "test".to_string();

    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (mut reader, mut writer) = stream.split();

    let mut buffer = Cursor::new(username.clone());
    writer.write_all_buf(&mut buffer).await?;
    writer.flush().await?;

    loop {
        let mut buffer = [0; 1024];
        tokio::select! {
            Ok(n) = reader.read(&mut buffer) => {
                if n == 0 {
                    break;
                }
                let msg = String::from_utf8_lossy(&buffer[..n]).to_string();
                println!("{}", msg);
            }
            _ = wait_for_message(io::stdin().lock()) => {
                let msg = format!("{}: {:?}", username, buffer);
                let mut buffer = Cursor::new(msg);
                writer.write_all_buf(&mut buffer).await?;
                writer.flush().await?;
            }
        };
    }

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

    #[tokio::test]
    async fn test_wait_for_message() {
        let input = b"mytestmessage\n";
        let reader = Cursor::new(input);

        let message = wait_for_message(reader).await.unwrap();

        assert_eq!(message, "mytestmessage");
    }
}
