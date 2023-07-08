use std::{
    io::Cursor,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc::{channel, Receiver},
};

mod structs;

use structs::client::Client;

async fn assign_username<R>(
    stream: &mut R,
    addr: SocketAddr,
    clients: Arc<Mutex<Vec<Client>>>,
) -> Result<String, Box<dyn std::error::Error>>
where
    R: AsyncRead + Unpin,
{
    let mut buffer = [0; 1024];
    let n = stream.read(&mut buffer).await?;
    let username = String::from_utf8_lossy(&buffer[..n]).trim().to_string();
    match username.is_empty() {
        true => Err("Username cannot be empty.".into()),
        false => {
            match clients.lock() {
                Ok(mut clients) => match clients.iter_mut().find(|c| c.addr_eq(addr)) {
                    Some(client) => client.set_username(username.clone()),
                    None => return Err("Client not found.".into()),
                },
                Err(_) => return Err("Error locking clients.".into()),
            };
            Ok(username)
        }
    }
}

async fn send_message(
    addr: SocketAddr,
    clients: Arc<Mutex<Vec<Client>>>,
    msg: String,
) -> Result<(), Box<dyn std::error::Error>> {
    clients
        .lock()
        .unwrap()
        .iter()
        .filter(|c| !c.addr_eq(addr))
        .for_each(|c| {
            let client = c.clone();
            let msg = msg.clone();
            tokio::spawn(async move {
                if let Err(e) = client.get_sender().send(msg).await {
                    println!("Error sending message to client: {}", e);
                }
            });
        });
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
        .find(|c| c.addr_eq(addr))
        .unwrap()
        .get_username()
        .clone();
    let msg = format!("{} has left the chat!", username);
    println!("{}", msg);
    send_message(addr, Arc::clone(&clients), msg).await?;
    clients.lock().unwrap().retain(|c| !c.addr_eq(addr));
    Ok(())
}

async fn handle_client(
    mut stream: TcpStream,
    addr: SocketAddr,
    clients: Arc<Mutex<Vec<Client>>>,
    mut receiver: Receiver<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = [0; 1024];

    handle_client_connect(addr, &mut stream, Arc::clone(&clients)).await?;

    loop {
        tokio::select! {
            Ok(n) = stream.read(&mut buffer) => {
                if n == 0 {
                    handle_client_disconnect(addr, Arc::clone(&clients)).await?;
                    break;
                }
                let msg = String::from_utf8_lossy(&buffer[..n]).to_string();
                println!("Message to be send to other threads: {}", msg.trim());
                send_message(addr, clients.clone(), msg).await?;
            }
            Some(msg) = receiver.recv() => {
                println!("Message to be send to client: {}", msg.trim());
                stream.write_all_buf(&mut Cursor::new(msg.trim())).await?;
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
            if let Err(e) = handle_client(stream, addr, Arc::clone(&clients), reciever).await {
                println!("Error handling client: {}", e);
                clients.lock().unwrap().retain(|c| !c.addr_eq(addr));
            }
        });
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

        let username = assign_username(&mut reader, addr, Arc::clone(&clients)).await;
        let client = clients
            .lock()
            .unwrap()
            .iter()
            .find(|c| c.addr_eq(addr))
            .unwrap()
            .clone();

        assert!(username.is_ok());
        assert_eq!(username.unwrap(), "mytestusername");
        assert_eq!(client.get_username(), "mytestusername");
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

    #[tokio::test]
    async fn test_clients_len_after_client_disconnect() {
        let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
        let clients: Arc<Mutex<Vec<Client>>> = Arc::new(Mutex::new(vec![Client::new(
            addr,
            channel::<String>(100).0,
        )]));

        let result = handle_client_disconnect(addr, Arc::clone(&clients)).await;

        assert!(result.is_ok());
        assert_eq!(clients.lock().unwrap().len(), 0);
    }
}
