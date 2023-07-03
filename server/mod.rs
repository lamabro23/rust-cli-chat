use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc::{channel, Receiver, Sender},
};

async fn handle_client(
    mut stream: TcpStream,
    addr: SocketAddr,
    clients: Arc<Mutex<HashMap<SocketAddr, Sender<String>>>>,
    mut receiver: Receiver<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];
    loop {
        tokio::select! {
            Ok(n) = stream.read(&mut buffer) => {
                if n == 0 {
                    break;
                }
                let msg = String::from_utf8_lossy(&buffer[..n]).to_string();
                let clients_clone = clients.lock().unwrap().clone();
                for (client_addr, client) in clients_clone.iter() {
                    if *client_addr != addr {
                        let client = client.clone();
                        let msg = msg.clone();
                        tokio::spawn(async move {
                            client.send(msg).await.unwrap();
                        });
                    }
                }
            }
            Some(msg) = receiver.recv() => {
                stream.write_all(msg.as_bytes()).await?;
            }
        };
    }

    Ok(())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let clients: Arc<Mutex<HashMap<SocketAddr, Sender<String>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (stream, addr) = listener.accept().await?;
        let (sender, reciever) = channel::<String>(100);

        let clients = Arc::clone(&clients);
        clients.lock().unwrap().insert(addr, sender);

        tokio::spawn(async move {
            handle_client(stream, addr, clients, reciever).await;
        });
    }
}
