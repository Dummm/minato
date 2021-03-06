use std::process::exit;
use std::option::Option;
use std::str::FromStr;
use regex::Regex;

extern crate structopt;
use structopt::{StructOpt, clap::crate_name};
use log::{info, error};
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde;

mod image;
mod image_manager;
mod container;
mod container_manager;
mod utils;
mod networking;
mod daemon;
mod client;
mod spec;


#[derive(Debug, StructOpt)]
#[structopt(global_setting = structopt::clap::AppSettings::ColoredHelp)]
pub struct Opt {
    #[structopt(short, long)]
    daemon: bool,
    #[structopt(short, long)]
    exit: bool,

    #[structopt(short = "D", long)]
    debug: bool,

    // #[structopt(short, long)]
    // networking: bool,

    #[structopt(short, long,
        env = "RUST_LOG",
        default_value = crate_name!())]
    log_level: String,

    #[structopt(subcommand)]
    subcommand: Option<Subcommand>
}
impl FromStr for Opt {
    type Err = std::string::ParseError;

    fn from_str(opt_str: &str) ->  Result<Self, Self::Err> {
        let regex_str = r####"(?:Opt \{ daemon: )(true|false)(?:, exit: )(true|false)(?:, debug: )(true|false)(?:, log_level: ")([A-Za-z]+\w)(?:", subcommand: )(None|.+)(?: \})"####;
        let regex = Regex::new(regex_str).unwrap();
        let matches = regex.captures(opt_str).unwrap();
        let daemon     = matches.get(1).map_or("", |m| m.as_str());
        let exit       = matches.get(2).map_or("", |m| m.as_str());
        let debug      = matches.get(3).map_or("", |m| m.as_str());
        let log_level  = matches.get(4).map_or("", |m| m.as_str());
        let subcommand = matches.get(5).map_or("", |m| m.as_str());

        let subcommand_conv = match Subcommand::from_str(subcommand) {
            Ok(s) => Some(s),
            Err(_) => None
        };

        Ok(Opt {
            daemon:     bool::from_str(daemon).unwrap(),
            exit:       bool::from_str(exit).unwrap(),
            debug:      bool::from_str(debug).unwrap(),
            log_level:  String::from(log_level),
            subcommand: subcommand_conv
        })
    }
}

#[derive(Debug, StructOpt)]
enum Subcommand {
    #[structopt(name = "image", about = "Manage images")]
    Image {
        #[structopt(subcommand, about = "pull|delete")]
        action: ImageAction
    },

    #[structopt(name = "container", about = "Manage containers")]
    Container {
        #[structopt(subcommand, about = "create|run|delete")]
        action: ContainerAction
    }
}
impl FromStr for Subcommand {
    type Err = std::io::Error;

    fn from_str(opt_str: &str) ->  Result<Self, Self::Err> {
        if opt_str == "None" {
            return Err(std::io::Error::new(std::io::ErrorKind::Other, "None"));
        }
        let regex_str = r####"((?:Some\()(.+)(?:\))|None)"####;
        let regex = Regex::new(regex_str).unwrap();
        let matches = regex.captures(opt_str).unwrap();
        let subcommand = matches.get(2).map_or("", |m| m.as_str());

        match subcommand.chars().next() {
            Some('I') => Ok(
                Subcommand::Image{
                    action: ImageAction::from_str(subcommand).unwrap()
                }
            ),
            Some('C') => Ok(
                Subcommand::Container{
                    action: ContainerAction::from_str(subcommand).unwrap()
                }
            ),
            _ => Err(
                std::io::Error::new(std::io::ErrorKind::Other, "invalid subcommand")
            )
        }
    }
}

#[derive(Debug, StructOpt)]
enum ImageAction {
    #[structopt(name = "pull", about = "Pull an image from the Docker repository")]
    Pull {
        #[structopt(name = "image-id",
            about = "Image ID in Docker repository",
            short = "i", long = "image-id",
            default_value = "library/alpine:latest")]
        image_id: String,
    },

    #[structopt(name = "list", about = "List pulled images")]
    List,

    #[structopt(name = "delete", about = "Delete an image from local storage")]
    Delete {
        #[structopt(name = "image-id",
            about = "Image ID in Docker repository",
            short = "i", long = "image-id")]
        image_id: String,
    }
}
impl FromStr for ImageAction {
    type Err = std::io::Error;

    fn from_str(opt_str: &str) ->  Result<Self, Self::Err> {
        let regex_str = r####"(?:Image \{ action: )(Pull|Delete)(?: \{ image_id: ")(.+)(?:" \} \})"####;
        let regex = Regex::new(regex_str).unwrap();
        let matches = regex.captures(opt_str).unwrap();
        let action = matches.get(1).map_or("", |m| m.as_str());
        let subcommand = matches.get(2).map_or("", |m| m.as_str());

        match action.chars().next() {
            Some('P') => Ok(
                ImageAction::Pull{
                    image_id: String::from(subcommand)
                }
            ),
            Some('D') => Ok(
                ImageAction::Delete{
                    image_id: String::from(subcommand)
                }
            ),
            _ => Err(
                std::io::Error::new(std::io::ErrorKind::Other, "invalid action")
            )
        }
    }
}

#[derive(Debug, StructOpt)]
enum ContainerAction {
    #[structopt(name = "create", about = "Create a container")]
    Create {
        #[structopt(name = "container-name",
            about = "Container name",
            short = "c", long = "container-nane")]
        container_name: String,

        #[structopt(name = "image-id",
            about = "Container name",
            short = "i", long = "image-id",
            default_value = "library/alpine:latest")]
        image_id: String,
    },

    #[structopt(name = "run", about = "Run a container")]
    Run {
        #[structopt(name = "container-name",
            about = "Container name",
            short = "c", long = "container-name")]
        container_name: String,

        #[structopt(name = "volume",
            about = "Bind volume to container",
            short = "v", long = "volume")]
        volume: Option<String>,

        #[structopt(name = "host-ip",
            about = "IP address for host-to-container communication",
            short = "H", long = "host-ip")]
        host_ip: Option<String>,

        #[structopt(name = "container-ip",
            about = "IP address for container-to-host communication",
            short = "C", long = "container-ip")]
        container_ip: Option<String>,
    },

    #[structopt(name = "open", about = "Open a container")]
    Open {
        #[structopt(name = "container-name",
            about = "Container name",
            short = "c", long = "container-name")]
        container_name: String,
    },

    #[structopt(name = "stop", about = "Stop a container")]
    Stop {
        #[structopt(name = "container-name",
            about = "Container name",
            short = "c", long = "container-name")]
        container_name: String,
    },

    #[structopt(name = "list", about = "List containers")]
    List,

    #[structopt(name = "delete", about = "Delete a container")]
    Delete {
        #[structopt(name = "container-name",
            about = "Container name",
            short = "c", long = "container-nane")]
        container_name: String,
    }
}
impl FromStr for ContainerAction {
    type Err = std::io::Error;

    fn from_str(opt_str: &str) ->  Result<Self, Self::Err> {
        let regex_str = r####"(?:Container \{ action: )(Create(?: \{ container_name: ")(.[^"]+)(?:", image_id: ")*(.[^"]+)|Run(?: \{ container_name: ")(.[^"]+)"(?:, volume: (Some\(".+"\)|None))(?:, host_ip: (Some\(".+"\)|None))(?:, container_ip: (Some\(".+"\)|None))|Stop(?: \{ container_name: ")(.[^"]+)"|Delete(?: \{ container_name: ")(.[^"]+)")(?: \} \})"####;
        let regex = Regex::new(regex_str).unwrap();
        let matches = regex.captures(opt_str).unwrap();
        let action                = matches.get(1).map_or("", |m| m.as_str());
        let create_container_name = matches.get(2).map_or("", |m| m.as_str());
        let create_image_id       = matches.get(3).map_or("", |m| m.as_str());
        let run_container_name    = matches.get(4).map_or("", |m| m.as_str());
        let run_volume            = matches.get(5).map_or("", |m| m.as_str());
        let run_host_ip           = matches.get(6).map_or("", |m| m.as_str());
        let run_container_ip      = matches.get(7).map_or("", |m| m.as_str());
        let stop_container_name   = matches.get(8).map_or("", |m| m.as_str());
        let delete_container_name = matches.get(9).map_or("", |m| m.as_str());

        match action.chars().next() {
            Some('C') => Ok(
                ContainerAction::Create {
                    container_name: String::from(create_container_name),
                    image_id:       String::from(create_image_id)
                }
            ),
            Some('R') => Ok(
                ContainerAction::Run {
                    container_name: String::from(run_container_name),
                    volume:         option_from_str(run_volume),
                    host_ip:        option_from_str(run_host_ip),
                    container_ip:   option_from_str(run_container_ip)
                }
            ),
            Some('S') => Ok(
                ContainerAction::Stop {
                    container_name: String::from(stop_container_name)
                }
            ),
            Some('D') => Ok(
                ContainerAction::Delete {
                    container_name: String::from(delete_container_name)
                }
            ),
            _ => Err(
                std::io::Error::new(std::io::ErrorKind::Other, "invalid action")
            )
        }
    }
}
fn option_from_str(option: &str) ->  Option<String> {
    let regex_str = r####"(?:Some\("(.+)"\))"####;
    let regex = Regex::new(regex_str).unwrap();
    let matches = regex.captures(option);
    if let None = matches {
        return None
    }
    let string = matches.unwrap()
        .get(1).map_or("", |m| m.as_str());
    return Some(String::from(string));
}


/**
 * * General
 *   TODO: Add .minato folder creation
 *   TODO: Add tini-static automatic download from github
 *   TODO: Add UI
 *   TODO: Fix unwraps so it doesn't panic
 *   TODO: Add comment to end of function
 *   TODO: Change back names from c's to n's
 *   TODO: Manage project
 *   TODO: Find a use for lib.rs file
 *
 * * Container
 *   TODO: Add container states (more code)
 *   TODO: Add contianer state check (i.e. before deletion)
 *   TODO: Remove dev mount and add ttys
 *   TODO: Populate 'sys' and 'dev' instead of mounting them from parent (maybe remove target)
 *   TODO: Pull containers from LXC repository
 *   TODO: Check if the inner fork is required or it works only with the execve
 *   * Namespaces
 *     TODO: Unshare user namespace later, separately
 *     TODO: Set uid and gids in user namespace
 *     TODO: Add namespace checks (i.e. check if userns is unshared)
 *   * CGroups
 *     ? /dev might have to be binded to the parent
 *     TODO: Try 'mount -t cgroup -o all cgroup /sys/fs/cgroup' to mount all cgroups faster
 *     TODO: Configure cgroups
 *   * Mounts
 *     TODO: Try archivemount instead of unarchiving layers
 *     TODO: Check if 'index=on' is needed when mounting overlayfs
 *   * Networking
 *     TODO: Fix networking
 *     TODO: Create a socket for each container
 *
 * * Daemon
 *   TODO: Check if the daemon does the clean-up steps before the child is stopped
 *   TODO: Fix daemon container closing
 *     ? Might be because no ttys
 *     ? Pid waiting is messed up
 *     ? Might neeed to move a fork on the daemon side
 *   TODO: Manage input and output from daemon
 *
 * * Image
 *   TODO: Add containers to image listing
 */

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    env_logger::builder()
        .format_indent(Some(4))
        .format_timestamp_millis()
        .init();

    let opt = Opt::from_args();
    // println!("{:?}", opt);

    // println!("{:?}", spec::Spec::load("src/config.json")?);

    if opt.daemon {
        info!("running in daemon mode");

        if opt.exit {
            info!("running as client");
            match client::Client::new() {
                Ok(client) => {
                    let message = format!("{:?}", opt);
                    client.send(message.as_bytes())?;
                    return Ok(())
                },
                Err(e) => {
                    info!("client creation failed");
                    info!("error: {}", e);
                    exit(1);
                }
            }
        }

        match opt.subcommand {
            None => {
                info!("running as daemon");

                match daemon::Daemon::new() {
                    Ok(daemon) => {
                        if let Err(e) = daemon.start() {
                            info!("error while starting daemon: {}", e);
                        };
                        Ok(())
                    },
                    Err(e) => {
                        info!("daemon creation failed");
                        info!("error: {}", e);
                        exit(1);
                    }
                }
            }
            Some(_) => {
                info!("running as client");

                match client::Client::new() {
                    Ok(client) => {
                        let message = format!("{:?}", opt);
                        client.send(message.as_bytes())?;
                        Ok(())
                    },
                    Err(e) => {
                        info!("client creation failed");
                        info!("error: {}", e);
                        exit(1);
                    }
                }
            }
        }
    } else {
        info!("running in daemonless mode");

        let image_manager = image_manager::ImageManager::new();
        let container_manager = container_manager::ContainerManager::new();

        if let Err(e) = utils::run_command(opt, &image_manager, &container_manager) {
            error!("program exited with error: {}", e);
        }
        Ok(())
    }

}
