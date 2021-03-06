use std::path::Path;
use std::fs::read_to_string;
use std::str::FromStr;
use nix::sys::stat::{Mode};
use nix::unistd::{mkdir};

use structopt::StructOpt;
// use dirs;
use log::debug;

use crate::*;
use crate::image::Image;
use crate::image_manager::ImageManager;
use crate::container::Container;
use crate::container_manager::ContainerManager;

#[allow(dead_code)]
/// Run a command for the managers, passed as a string
pub fn run_command_from_string(opt_str: &str, image_manager: &ImageManager, container_manager: &ContainerManager) -> Result<(), Box<dyn std::error::Error>> {
    let opt: Opt = Opt::from_str(opt_str).unwrap();
    run_command(opt, image_manager, container_manager)?;
    Ok(())
}
/// Run a command for the managers
pub fn run_command(opt: Opt, image_manager: &ImageManager, container_manager: &ContainerManager) -> Result<(), Box<dyn std::error::Error>> {
    match opt.subcommand {
        Some(Subcommand::Image  { action }) => match action {
            ImageAction::Pull   { image_id } => image_manager.pull(&image_id),
            ImageAction::List                => image_manager.list(),
            ImageAction::Delete { image_id } => image_manager.delete(&image_id),
        },
        Some(Subcommand::Container  { action }) => match action {
            ContainerAction::Create { container_name, image_id } => container_manager.create(&container_name, &image_id),
            ContainerAction::Run    { container_name, volume, host_ip, container_ip }   => container_manager.run(&container_name, opt.daemon, volume, host_ip, container_ip),
            ContainerAction::Open   { container_name }           => container_manager.open(&container_name),
            ContainerAction::Stop   { container_name }           => container_manager.stop(&container_name),
            ContainerAction::List                                => container_manager.list(),
            ContainerAction::Delete { container_name }           => container_manager.delete(&container_name),
        }
        None => {
            info!("unexpected arguments");
            Opt::clap().print_help().unwrap();
            exit(1);
        }
    }
}


/// Add missing tags to image id
pub fn fix_image_id(image_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut id = String::from(image_id);

    if !id.contains(':') {
        id.push_str(":latest");
    }

    if !id.contains('/') {
        id = format!("library/{}", id);
    }

    Ok(id)
}
/// Parse image id
pub fn split_image_id(image_id: String) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut ids: Vec<String> = image_id.split(':').map(|str| String::from(str)).collect();
    if ids.len() == 1 {
        ids.push(String::from("latest"));
    }


    if !ids[0].contains('/') {
        ids[0] = format!("library/{}", ids[0]);
    }

    Ok((ids[0].clone(), ids[1].clone()))
}
/// Get path to image, from the image id
pub fn get_image_path_with_str(image_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    // let home = match dirs::home_dir() {
    //     Some(path) => path,
    //     None       => return Err("error getting home directory".into())
    // };

    // Ok(format!(
    //     "{}/.minato/images/{}",
    //     home.display(), image_id
    // ))
    Ok(format!(
        "/var/lib/minato/images/{}",
        image_id
    ))
}
/// Get path to image, from image object
pub fn get_image_path(image: &Image) -> Result<String, Box<dyn std::error::Error>> {
    get_image_path_with_str(image.id.as_str())
}


/// Get path to container, from the container id
pub fn get_container_path_with_str(container_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    // let home = match dirs::home_dir() {
    //     Some(path) => path,
    //     None       => return Err("error getting home directory".into())
    // };

    // Ok(format!(
    //     "{}/.minato/containers/{}",
    //     home.display(), container_id
    // ))
    Ok(format!(
        "/var/lib/minato/containers/{}",
        container_id
    ))
}
/// Get path to container, from imagcontainere object
pub fn get_container_path(container: &Container) -> Result<String, Box<dyn std::error::Error>> {
    get_container_path_with_str(container.id.as_str())
}
/// Get container pid, from the container id
pub fn get_container_pid_with_str(container_id: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let container_path = utils::get_container_path_with_str(container_id)?;

    let pid_path = format!("{}/pid", container_path);
    if !Path::new(&pid_path).exists() {
        return Ok(None);
    }

    let pid = read_to_string(pid_path)?;

    Ok(Some(pid.replace('\n', "")))
}
/// Get container pid, from the container object
pub fn get_container_pid(container: &Container) -> Result<Option<String>, Box<dyn std::error::Error>> {
    get_container_pid_with_str(container.id.as_str())
}
/// Prepare container directory by removing it, if it exists, and recreating it with specified permissions
pub fn prepare_directory(rootfs: &str, dir_name: &str, perms: Mode) -> Result<(), Box<dyn std::error::Error>> {
    let dir_path = Path::new(rootfs).join(dir_name);

    if dir_path.exists() {
        info!("removing old '{}' folder...", dir_path.display());
        std::fs::remove_dir_all(&dir_path)?;
    }

    info!("making new '{}' folder...", dir_name);
    mkdir(
        &dir_path,
        perms
    )?;

    Ok(())
}

#[allow(dead_code)]
/// Print capabilities
pub fn print_caps() {
    let output = std::process::Command::new("/usr/sbin/capsh")
            .arg("--print")
            // .arg("| grep Current")
            .output();
    if let Err(e) = output {
        debug!("error printing capabilities: {}", e);
    } else {
        debug!("capabilities: \n{}", String::from_utf8_lossy(&output.unwrap().stdout));
    }
}

/// Get path to socket
pub fn get_socket_path(socket_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    // let home = match dirs::home_dir() {
    //     Some(path) => path,
    //     None       => return Err("error getting home directory".into())
    // };

    // Ok(format!(
    //     "{}/.minato/{}",
    //     home.display(), socket_name
    // ))
    Ok(format!(
        "/var/lib/minato/{}",
        socket_name
    ))
}