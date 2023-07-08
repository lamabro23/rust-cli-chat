use std::io::{self, stdin, BufRead, Cursor};

use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
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

#[tokio::main]
async fn main() -> io::Result<()> {
    let username = request_username(&mut io::stdin().lock()).unwrap();

    let stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (mut reader, mut writer) = stream.into_split();

    send_message(&mut writer, username).await?;

    tokio::spawn(async move {
        loop {
            let mut buffer = vec![0; 1024];
            match reader.read(&mut buffer).await {
                Ok(n) if n > 0 => {
                    let msg = String::from_utf8_lossy(&buffer).to_string();
                    println!("{}", msg);
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => (),
                Err(_) | Ok(0) => {
                    println!("Connection with server was severed.");
                    break;
                }
                Ok(_) => (),
            }
        }
    });

    loop {
        let mut buff = String::new();
        stdin()
            .read_line(&mut buff)
            .expect("Reading from stdin failed!");
        send_message(&mut writer, buff).await?;
    }
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
