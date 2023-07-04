use std::io::Cursor;

use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

fn request_username() -> String {
    println!("Please enter your username.");
    let mut username = String::new();
    std::io::stdin()
        .read_line(&mut username)
        .expect("Failed to read line");
    username.trim().to_string()
}

async fn wait_for_message() -> String {
    let mut message = String::new();
    std::io::stdin()
        .read_line(&mut message)
        .expect("Failed to read line");
    message.trim().to_string()
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // let username = request_username();
    let username = "test".to_string();

    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (reader, mut writer) = stream.split();

    let mut buffer = Cursor::new(username.clone());
    writer.write_all_buf(&mut buffer).await?;
    writer.flush().await?;

    loop {
        println!("Please enter a message.");
        let message = wait_for_message().await;
        if !message.is_empty() {
            println!("sending message: {}", message);
            let mut buffer = Cursor::new(message);
            writer.write_all_buf(&mut buffer).await?;
            writer.flush().await?;
        }
        println!("waiting for message...");
        reader.readable().await?;
        println!("message received!");

        let mut buf = Vec::with_capacity(1024);

        match reader.try_read_buf(&mut buf) {
            Ok(0) => break,
            Ok(_) => {}
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                println!("failed to read from socket; err = {:?}", e);
                return Err(e.into());
            }
        }

        println!("{}", String::from_utf8(buf).unwrap());
    }

    Ok(())
}
