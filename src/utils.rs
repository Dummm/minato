use std::path::Path;
use std::str::FromStr;
use nix::sys::stat::{Mode};
use nix::unistd::{mkdir};

use structopt::StructOpt;
use dirs;

use crate::*;
use crate::image::Image;
use crate::image_manager::ImageManager;
use crate::container::Container;
use crate::container_manager::ContainerManager;

#[allow(dead_code)]
pub fn run_command_from_string(opt_str: &str, image_manager: &ImageManager, container_manager: &ContainerManager) -> Result<(), Box<dyn std::error::Error>> {
    let opt: Opt = Opt::from_str(opt_str).unwrap();
    run_command(opt, image_manager, container_manager)?;
    Ok(())
}

pub fn run_command(opt: Opt, image_manager: &ImageManager, container_manager: &ContainerManager) -> Result<(), Box<dyn std::error::Error>> {
    match opt.subcommand {
        Some(Subcommand::Image  { action }) => match action {
            ImageAction::Pull   { image_id } => image_manager.pull(&image_id),
            ImageAction::Delete { image_id } => image_manager.delete(&image_id),
        },
        Some(Subcommand::Container  { action }) => match action {
            ContainerAction::Create { container_name, image_id } => container_manager.create(&container_name, &image_id),
            ContainerAction::Run    { container_name }           => container_manager.run(&container_name),
            ContainerAction::Delete { container_name }           => container_manager.delete(&container_name),
        }
        None => {
            info!("unexpected arguments");
            Opt::clap().print_help().unwrap();
            exit(1);
        }
    }
}

pub fn get_image_path_with_str(image_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };

    Ok(format!(
        "{}/.minato/images/{}",
        home.display(), image_id
    ))
}

pub fn get_socket_path(socket_name: &str) -> Result<String, Box<dyn std::error::Error>> {
    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };

    Ok(format!(
        "{}/.minato/{}",
        home.display(), socket_name
    ))
}

pub fn get_image_path(image: &Image) -> Result<String, Box<dyn std::error::Error>> {
    get_image_path_with_str(image.id.as_str())
}

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


pub fn get_container_path_with_str(container_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };

    Ok(format!(
        "{}/.minato/containers/{}",
        home.display(), container_id
    ))
}

pub fn get_container_path(container: &Container) -> Result<String, Box<dyn std::error::Error>> {
    get_container_path_with_str(container.id.as_str())
}

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
