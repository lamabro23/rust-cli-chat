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

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // let username = request_username();
    let username = "test".to_string();

    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    let (reader, mut writer) = stream.split();

    let mut buffer = Cursor::new(username);
    writer.write_all_buf(&mut buffer).await?;
    writer.flush().await?;

    loop {
        reader.readable().await?;

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
