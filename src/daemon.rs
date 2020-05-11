// use std::iter;
// use rand::{thread_rng, Rng};
// use rand::distributions::Alphanumeric;
use std::path::Path;
use std::os::unix::net::{UnixStream, UnixListener};
use std::io::prelude::*;
use std::fs;
use std::str::FromStr;

use log::info;

use crate::utils;
use crate::image_manager::ImageManager;
use crate::container_manager::ContainerManager;
use crate::Opt;


pub struct Daemon<'a> {
    listener: UnixListener,
    image_manager: ImageManager<'a>,
    container_manager: ContainerManager<'a>
}
// impl Drop for Daemon {
//     fn drop(&mut self) {
//         info!("deleting socket...");
//         let socket = Path::new(&self.socket_path);
//         if socket.exists() {
//             fs::remove_file(&socket);
//         }
//     }
// }
impl<'a> Daemon<'a> {
    pub fn new() -> Result<Daemon<'a>, Box<dyn std::error::Error>> {
        info!("creating daemon...");
        // let mut rng = thread_rng();
        // let suffix = iter::repeat(())
        //     .map(|()| rng.sample(Alphanumeric))
        //     .take(4)
        //     .collect::<String>();
        // let suffix = process::id();
        // let s_name = format!("socket_{}", suffix);
        let s_name = String::from("socket");
        let s_path_str = utils::get_socket_path(s_name.as_str()).unwrap();
        let socket = Daemon::create_socket(s_path_str.clone())?;

        Ok(Daemon {
            listener: socket,
            image_manager: ImageManager::new(),
            container_manager: ContainerManager::new()
        })
    }

    fn create_socket(socket_path: String) -> Result<UnixListener, Box<dyn std::error::Error>> {
        info!("creating socket...");

        let socket = Path::new(&socket_path);
        if socket.exists() {
            info!("socket file already exists. removing...");
            fs::remove_file(&socket)?;
        }

        let listener = UnixListener::bind(&socket)?;
        let addr = listener.local_addr()?;
        info!("listener local address: {:?}", addr);

        Ok(listener)
    }

    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("starting daemon...");

        info!("waiting for client...");
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    info!("client found....");
                    self.handle_client(stream)?;
                }
                Err(err) => {
                    info!("error encountered: {}", err);
                    info!("stopping daemon...");
                    break;
                }
            }
        }

        Ok(())
    }

    // pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
    //     info!("stopping daemon...");
    //     Ok(())
    // }

    fn handle_client(&self, stream: UnixStream) -> Result<(), Box<dyn std::error::Error>> {
        info!("handling client...");

        let mut temp_stream = stream.try_clone()?;

        info!("reading message...");
        let mut buffer = [0; 1024];
        let size = temp_stream.read(&mut buffer).unwrap();
        let message = String::from_utf8(buffer[..size].to_vec()).unwrap();
        info!("client message: {}", message);

        let opt = Opt::from_str(message.as_str())?;
        info!("opt: {:?}", opt);
        info!("executing command ...");
        utils::run_command(opt, &self.image_manager, &self.container_manager)?;

        info!("sending response...");
        temp_stream.write_all(b"OK")?;

        info!("cliend handled succesfully!");
        Ok(())
    }
}