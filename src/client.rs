use std::os::unix::net::UnixStream;
use std::io::prelude::*;

use log::info;

use crate::utils;

pub struct Client {
    stream: UnixStream
}
impl Client {
    pub fn new() -> Result<Client, Box<dyn std::error::Error>> {
        info!("creating client...");

        let s_name = String::from("socket");
        let s_path_str = utils::get_socket_path(s_name.as_str()).unwrap();
        let socket = Client::connect_to_socket(s_path_str.clone())?;

        Ok(Client {
            // socket_name: s_name,
            // socket_path: s_path_str,
            stream: socket
        })
    }

    fn connect_to_socket(socket_path: String) -> Result<UnixStream, Box<dyn std::error::Error>> {
        info!("creating socket...");

        let stream = UnixStream::connect(socket_path)?;
        let addr = stream.local_addr()?;
        info!("listener local address: {:?}", addr);

        Ok(stream)
    }

    pub fn send(&self, message: &[u8]) -> Result<(), Box<dyn std::error::Error>> {

        let mut temp_stream = self.stream.try_clone()?;

        info!("sending message...");
        temp_stream.write_all(message)?;

        info!("reading response...");
        let mut buffer = [0; 1024];
        let size = temp_stream.read(&mut buffer).unwrap();
        let response = String::from_utf8(buffer[..size].to_vec()).unwrap();
        info!("daemon response: {}", response);

        Ok(())
    }
}