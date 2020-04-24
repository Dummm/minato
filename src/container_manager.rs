use std::fs;
use std::io::{self, prelude::*};
use std::path::Path;
use std::os::unix;
use std::process::Command;
use std::time;
use std::thread;
use std::ffi::CString;
use nix::mount::*;
use nix::sched::*;
use nix::unistd::{chdir, execve, mkdir, pivot_root, sethostname};
use nix::sys::{stat, wait::waitpid};

use dirs;
use log::{info, error};
use clap::ArgMatches;

use crate::image::Image;
use crate::image_manager;
use crate::container::Container;
use crate::utils;


fn create_directory_structure(container: &Container) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating container directory structure...");

    let container_path_str = utils::get_container_path(container)?;
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
            utils::get_image_path(&container.image.as_ref().unwrap()).unwrap(),
            container_lower_path
        )?;
    }

    Ok(())
}

// TODO: Generate config.json file
fn generate_config_json() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn create_with_args(args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let image_name = args.value_of("image_name").unwrap();
    let container_name = args.value_of("container_name").unwrap();
    create(container_name, image_name)
}

pub fn create(container_name: &str, image_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    let image = match Image::load(image_id)? {
        Some(image) => image,
        None        => {
            // TODO: Add verification
            info!("image not found. trying to pull image...");
            image_manager::pull(image_id)?;
            create(container_name, image_id)?;
            return Ok(())
        }
    };
    let container = Container::new(Some(container_name), Some(image));

    info!("creating container '{}'...", container.id);

    let container_path_str = utils::get_container_path_with_str(container_name)?;
    if Path::new(container_path_str.as_str()).exists() {
        info!("container exists. skipping creation...");
        return Ok(())
    }

    generate_config_json()?;
    create_directory_structure(&container)?;

    Ok(())
}

fn mount_container_filesystem(container: &Container)  -> Result<(), Box<dyn std::error::Error>> {
    info!("mounting container filesystem...");

    let container_path_str = utils::get_container_path(container)?;
    let container_path = Path::new(container_path_str.as_str());

    // TODO: Fix this mess
    let mut lowerdir_arg = "lowerdir=".to_string();
    let subdirectories = container_path.join("lower").read_dir()?;
    let subdirectories_vector = subdirectories
        .map(|dir| format!("{}", dir.unwrap().path().display()))
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

// TODO: Find a better way to find image
pub fn load_container(container_name: &str) -> Result<Option<Container>, Box<dyn std::error::Error>> {
    let mut container = Container::new(Some(container_name), None);
    let container_path_str = utils::get_container_path(&container)?;
    let container_path = Path::new(container_path_str.as_str());

    if !container_path.exists() {
        return Ok(None)
    }

    let container_lower_path = container_path.join("lower");
    let container_image_path = container_lower_path.read_link().unwrap();

    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };
    let images_path = format!(
        "{}/.minato/images/",
        home.display()
    );
    info!("{}", images_path);

    let image_id = container_image_path
        .strip_prefix(images_path)
        .unwrap()
        .to_str()
        .unwrap();
    // info!("image name: {}", image_name);
    let image = match Image::load(image_id).unwrap() {
        Some(image) => image,
        None        => return Ok(None)
    };
    container.image = Some(image);

    Ok(Some(container))
}

fn prepare_parent_filesystems() -> Result<(), Box<dyn std::error::Error>> {
    info!("making host mount namespace private...");
    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None::<&str>,
    )?;

    Ok(())
}

// TODO: Check if stack values modifications are required
fn start_container_process(container: &Container) -> Result<(), Box<dyn std::error::Error>> {
    info!("cloning process...");

    let rootfs_path_str = format!(
        "{}/merged",
        utils::get_container_path(container)?
    );

    let cb = Box::new(|| {
        if let Err(e) = init(rootfs_path_str.as_str()) {
            error!("unable to initialize container: {}", e);
            -1
        } else {
            0
        }
    });

    let stack = &mut [0; 1024 * 1024];
    let clone_flags =
        CloneFlags::CLONE_NEWUTS |
        CloneFlags::CLONE_NEWPID |
        CloneFlags::CLONE_NEWNS |
        CloneFlags::CLONE_NEWIPC |
        CloneFlags::CLONE_NEWNET;
    let childpid = clone(
        cb,
        stack,
        clone_flags,
        None
    )?;

    info!("child pid: {}", childpid);
    thread::sleep(time::Duration::from_millis(300));
    waitpid(childpid, None)?;

    Ok(())
}

pub fn run_with_args(args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let container_name = args.value_of("container_name").unwrap();
    run(container_name)
}

pub fn run(container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let container = match load_container(container_name).unwrap() {
        Some(container) => container,
        None            => {
            info!("container not found. exiting...");
            return Ok(())
        }
    };
    info!("running container '{}'...", &container.id);

    mount_container_filesystem(&container)?;

    // Unmount mounted filesystem in case of error
    if let Err(e) = prepare_parent_filesystems() {
        cleanup(&container)?;
        return Err(e);
    };
    if let Err(e) = start_container_process(&container) {
        cleanup(&container)?;
        return Err(e);
    };

    cleanup(&container)?;

    info!("run successfull");
    Ok(())
}

// TODO: Change from hostname and cmd from literals to variables
fn init(rootfs: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("initiating container...");

    info!("making root private...");
    mount(
        Some(rootfs),
        rootfs,
        None::<&str>,
        MsFlags::MS_BIND | MsFlags::MS_REC,
        None::<&str>,
    )?;

    info!("removing old 'put_old' folder...");
    let prev_rootfs = Path::new(rootfs).join("put_old");
    if prev_rootfs.exists() {
        std::fs::remove_dir_all(&prev_rootfs)?;
    }

    info!("making new 'put_old' folder...");
    mkdir(
        &prev_rootfs,
        stat::Mode::S_IRWXU | stat::Mode::S_IRWXG | stat::Mode::S_IRWXO,
    )?;

    info!("pivoting root...");
    pivot_root(rootfs, &prev_rootfs)?;
    chdir("/")?;
    umount2("/put_old", MntFlags::MNT_DETACH)?;

    info!("mounting proc...");
    mount(
        Some("proc"),
        "/proc",
        Some("proc"),
        MsFlags::MS_NOEXEC | MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_RELATIME,
        None::<&str>,
    )?;

    info!("mounting tmpfs...");
    mount(
        Some("tmpfs"),
        "/dev",
        Some("tmpfs"),
        MsFlags::MS_NOSUID | MsFlags::MS_RELATIME,
        None::<&str>,
    )?;

    sethostname("test")?;
    do_exec("/bin/sh")?;
    // do_exec("/bin/bash")?;

    Ok(())
}

fn do_exec(cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("preparing command execution...");

    let args = &[Path::new(cmd).file_stem().unwrap().to_str().unwrap()];
    let envs = &["PATH=/bin:/sbin:/usr/bin:/usr/sbin"];
    let p = CString::new(cmd).unwrap();

    let a: Vec<CString> = args.iter()
        .map(|s| CString::new(s.to_string()).unwrap_or_default())
        .collect();
    let e: Vec<CString> = envs.iter()
        .map(|s| CString::new(s.to_string()).unwrap_or_default())
        .collect();

    info!("executing command...");
    info!("{:?}", args);
    info!("{:?}", envs);
    info!("{:?}", p);
    execve(&p, &a, &e)?;

    Ok(())
}

fn unmount_container_filesystem(container: &Container) -> Result<(), Box<dyn std::error::Error>> {
    let container_path_str = utils::get_container_path(container)?;
    let merged = format!("{}/merged", container_path_str);
    umount2(merged.as_str(), MntFlags::MNT_DETACH)?;

    Ok(())
}

fn cleanup(container: &Container) -> Result<(), Box<dyn std::error::Error>> {
    info!("cleaning up container...");

    unmount_container_filesystem(container)?;

    Ok(())
}

pub fn delete_with_args(args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let container_name = args.value_of("container_name").unwrap();
    delete(container_name)
}

// TODO: Add contianer state check
pub fn delete(container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("deleting container '{}'...", container_name);

    let container_path_str = utils::get_container_path_with_str(container_name)?;
    let container_path = Path::new(container_path_str.as_str());

    if !container_path.exists() {
        info!("container not found. skipping deletion...");
        return Ok(())
    }

    fs::remove_dir_all(container_path)?;

    info!("deletion successfull");
    Ok(())
}