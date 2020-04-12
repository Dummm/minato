use std::fs;
use std::io::prelude::*;
use std::io;
use std::iter;
use std::path::Path;
use std::os::unix;
use std::process::Command;

use dirs;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use log::info;

use super::image::Image;
use super::image_manager;

pub struct Container {
    pub id: String,
    pub image: Option<Image>,
    pub state: State,
}

#[derive(Debug, PartialEq)]
pub enum State {
    Creating,
    Created(u32),
    Running(u32),
    Stopped,
}

impl Container {
    pub fn new(image: Option<Image>, container_id: Option<&str>) -> Container {
        let id: String = match container_id {
            Some(id) => id.to_string(),
            None => {
                let mut rng = thread_rng();
                iter::repeat(())
                    .map(|()| rng.sample(Alphanumeric))
                    .take(8)
                    .collect::<String>()
            }
        };

        Container {
            id,
            image,
            state: State::Stopped,
        }
    }
}


pub fn create_directory_structure(container: &Container) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating container directory structure...");

    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };

    let container_path_str = format!(
        "{}/.minato/containers/{}",
        home.display(), container.id
    );
    let container_path = Path::new(container_path_str.as_str());
    if !container_path.exists() {
        fs::create_dir_all(container_path.clone())?;

        let subdirectories = vec!["upper", "work", "merged"];
        for subdirectory in subdirectories {
            let subdirectory_path = container_path.join(subdirectory);
            fs::create_dir_all(subdirectory_path.clone())?;
        }

        let container_lower_path = container_path.join("lower");
        unix::fs::symlink(
            image_manager::get_image_path(&container.image.as_ref().unwrap()).unwrap(),
            container_lower_path
        )?;
    }

    Ok(())
}

// TODO: Generate config.json file
fn generate_config_json() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn create(container: &Container) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating container '{}'...", container.id);

    generate_config_json()?;
    create_directory_structure(container)?;

    Ok(())
}

fn mount_container_filesystem(container: &Container)  -> Result<(), Box<dyn std::error::Error>> {
    info!("mounting container filesystem...");

    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };
    let container_path_str = format!(
        "{}/.minato/containers/{}",
        home.display(), container.id
    );
    let container_path = Path::new(container_path_str.as_str());

    // TODO: Fix this mess
    let mut lowerdir_arg = "lowerdir=".to_string();
    let subdirectories = container_path.join("lower").read_dir()?;
    let subdirectories_vector = subdirectories
        .map(|dir| format!("{}", dir.unwrap().path().display()))
        // .map(|dir| format!("lower/{:?}", dir.unwrap().file_name()))
        .collect::<Vec<String>>();
    let subdirectories_str = subdirectories_vector.join(":");
    lowerdir_arg.push_str(subdirectories_str.as_str());

    let upperdir_arg  = format!("upperdir={}/upper", container_path_str);
    let workdir_arg   = format!("workdir={}/work", container_path_str);
    let mergeddir_arg = format!("{}/merged", container_path_str);

    let full_arg = format!("-o{},{},{}",
        lowerdir_arg, upperdir_arg, workdir_arg
    );
    info!("using mount arguments: \n{}\n{}", full_arg, mergeddir_arg);

    let output = Command::new("./fuse-overlayfs/fuse-overlayfs")
        .arg(full_arg)
        .arg(mergeddir_arg)
        .output()
        .unwrap();

    info!("mount output {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    Ok(())
}

pub fn run(container: &Container) -> Result<(), Box<dyn std::error::Error>> {
    info!("running container '{}'...", container.id);

    mount_container_filesystem(container)?;

    Ok(())
}



