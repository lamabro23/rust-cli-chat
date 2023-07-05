use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc::{channel, Receiver, Sender},
};

async fn assign_username<R: AsyncRead + Unpin>(
    stream: &mut R,
    addr: SocketAddr,
    clients: Arc<Mutex<Vec<Client>>>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).await?;
    let username = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
    match username.is_empty() {
        // TODO: Send response to the client
        true => Err("Username cannot be empty.".into()),
        false => {
            clients
                .lock()
                .unwrap()
                .iter_mut()
                .find(|c| c.addr == addr)
                .unwrap()
                .set_username(username.clone());
            Ok(username)
        }
    }
}

async fn send_message(
    addr: SocketAddr,
    clients: Arc<Mutex<Vec<Client>>>,
    msg: String,
) -> Result<(), Box<dyn std::error::Error>> {
    for client in clients.lock().unwrap().iter() {
        if client.addr != addr {
            let client = client.clone();
            let msg = msg.clone();
            tokio::spawn(async move {
                if let Err(e) = client.sender.send(msg).await {
                    println!("Error sending message to client: {}", e);
                }
            });
        }
    }
    Ok(())
}

async fn handle_client_connect(
    addr: SocketAddr,
    stream: &mut TcpStream,
    clients: Arc<Mutex<Vec<Client>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let username = assign_username(stream, addr, Arc::clone(&clients)).await?;
    let msg = format!("{} has joined the chat!", username);
    println!("{}", msg);
    send_message(addr, clients, msg).await?;
    Ok(())
}

async fn handle_client_disconnect(
    addr: SocketAddr,
    clients: Arc<Mutex<Vec<Client>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let username = clients
        .lock()
        .unwrap()
        .iter()
        .find(|c| c.addr == addr)
        .unwrap()
        .username
        .clone();
    let msg = format!("{} has left the chat!", username);
    println!("{}", msg);
    send_message(addr, clients, msg).await?;
    Ok(())
}

async fn handle_client(
    mut stream: TcpStream,
    addr: SocketAddr,
    clients: Arc<Mutex<Vec<Client>>>,
    mut receiver: Receiver<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];

    handle_client_connect(addr, &mut stream, clients.clone()).await?;

    loop {
        tokio::select! {
            Ok(n) = stream.read(&mut buffer) => {
                if n == 0 {
                    handle_client_disconnect(addr, clients.clone()).await?;
                    clients.lock().unwrap().retain(|c| c.addr != addr);
                    break;
                }
                let msg = String::from_utf8_lossy(&buffer[..n]).to_string();
                println!("1: Sending message to client: {}", msg);
                send_message(addr, clients.clone(), msg).await?;
            }
            Some(msg) = receiver.recv() => {
                println!("2: Sending message to client: {}", msg);
                stream.write_all(msg.as_bytes()).await?;
            }
        };
    }

    Ok(())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    let clients: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(Vec::new()));

    loop {
        let (stream, addr) = listener.accept().await?;
        let (sender, reciever) = channel::<String>(100);

        let clients = Arc::clone(&clients);
        clients.lock().unwrap().push(Client::new(addr, sender));

        tokio::spawn(async move {
            if let Err(e) = handle_client(stream, addr, clients, reciever).await {
                println!("Error handling client: {}", e);
            }
        });
    }
}

#[derive(Clone, Debug)]
struct Client {
    username: String,
    addr: SocketAddr,
    sender: Sender<String>,
}

impl Client {
    fn new(addr: SocketAddr, sender: Sender<String>) -> Self {
        Self {
            username: String::new(),
            addr,
            sender,
        }
    }

    fn set_username(&mut self, username: String) {
        self.username = username;
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::*;

    #[tokio::test]
    async fn test_assign_username_success() {
        let input = b"mytestusername\n";
        let mut reader = Cursor::new(input);
        let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
        let clients: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(vec![Client::new(
            addr,
            channel::<String>(100).0,
        )]));

        let username = assign_username(&mut reader, addr, clients).await;

        assert!(username.is_ok());
        assert_eq!(username.unwrap(), "mytestusername");
    }

    #[tokio::test]
    async fn test_assign_username_empty() {
        let input = b"\n";
        let mut reader = Cursor::new(input);
        let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
        let clients: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(vec![Client::new(
            addr,
            channel::<String>(100).0,
        )]));

        let username = assign_username(&mut reader, addr, clients).await;

        assert!(username.is_err());
    }
}
