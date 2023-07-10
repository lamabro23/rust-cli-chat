use std::net::SocketAddr;

use tokio::sync::mpsc::Sender;

#[derive(Clone, Debug)]
pub(crate) struct Client {
    username: String,
    addr: SocketAddr,
    sender: Sender<String>,
}

impl Client {
    pub(crate) fn new(addr: SocketAddr, sender: Sender<String>) -> Self {
        Self {
            username: String::new(),
            addr,
            sender,
        }
    }

    pub(crate) fn set_username(&mut self, username: String) {
        self.username = username;
    }

    pub(crate) fn get_username(&self) -> &String {
        &self.username
    }

    pub(crate) fn get_sender(&self) -> &Sender<String> {
        &self.sender
    }

    pub fn addr_eq(&self, addr: SocketAddr) -> bool {
        self.addr == addr
    }
}
