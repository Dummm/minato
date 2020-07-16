// use std::iter;
// use rand::{thread_rng, Rng};
// use rand::distributions::Alphanumeric;
use std::path::Path;
use std::os::unix::net::{UnixStream, UnixListener};
use std::io::prelude::*;
use std::fs;
use std::str::FromStr;
use nix::unistd::getpid;
use std::error::Error;
use std::fmt;

use log::info;

use crate::utils;
use crate::image_manager::ImageManager;
use crate::container_manager::ContainerManager;
use crate::Opt;

#[derive(Debug)]
struct ExitError(String);
impl Error for ExitError {}
impl fmt::Display for ExitError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "There is an error: {}", self.0)
    }
}


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
    /// Create a new daemon object
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

        info!("created daemon.");
        Ok(Daemon {
            listener: socket,
            image_manager: ImageManager::new(),
            container_manager: ContainerManager::new()
        })
    }

    /// Create a socket to listen to commands from clients
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

        info!("created socket.");
        Ok(listener)
    }

    /// Start the daemon by listening for commands through the socket
    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("starting daemon...");

        // let home = match dirs::home_dir() {
        //     Some(path) => path,
        //     None       => return Err("error getting home directory".into())
        // };

        // let pid_path = format!("{}/.minato/pid", home.display());
        let pid_path = format!("/var/lib/minato/pid");
        if Path::new(&pid_path).exists() {
            info!("removing pid file...");
            fs::remove_file(&pid_path)?;
        }
        let pid_str = format!("{}\n", getpid().as_raw().to_string());
        fs::File::create(&pid_path)?;
        fs::write(&pid_path, pid_str)?;

        info!("waiting for client...");
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    info!("client found....");

                    match self.handle_client(stream) {
                        Ok(_) => {},
                        Err(e) =>  {
                            info!("error handling client: {}", e);
                            // info!("stopping daemon...");
                            break;
                        }
                    };
                }
                Err(err) => {
                    info!("error encountered: {}", err);
                    info!("stopping daemon...");
                    break;
                }
            }
        }

        if Path::new(&pid_path).exists() {
            info!("removing pid file...");
            fs::remove_file(&pid_path)?;
        }

        Ok(())
    }

    // pub fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
    //     info!("stopping daemon...");
    //     Ok(())
    // }

    /// Convert string command recieved from client and execute it
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

        if opt.exit {
            return Err(Box::new(ExitError("close daemon".into())));
        }

        info!("executing command ...");
        utils::run_command(opt, &self.image_manager, &self.container_manager)?;

        info!("sending response...");
        temp_stream.write_all(b"OK")?;

        info!("handled cliend.");
        Ok(())
    }
}