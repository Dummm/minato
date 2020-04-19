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

use crate::image_manager;
use crate::container::Container;

// TODO: Move to utils/helpers
pub fn get_container_path(container: &Container) -> Result<String, Box<dyn std::error::Error>> {
    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };

    Ok(format!(
        "{}/.minato/containers/{}",
        home.display(), container.id
    ))
}

fn create_directory_structure(container: &Container) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating container directory structure...");

    let container_path_str = get_container_path(container)?;
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

    let container_path_str = get_container_path(container)?;
    if Path::new(container_path_str.as_str()).exists() {
        info!("container exists. skipping creation...");
        return Ok(())
    }

    generate_config_json()?;
    create_directory_structure(container)?;

    Ok(())
}

fn mount_container_filesystem(container: &Container)  -> Result<(), Box<dyn std::error::Error>> {
    info!("mounting container filesystem...");

    let container_path_str = get_container_path(container)?;
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

pub fn run(container: &Container) -> Result<(), Box<dyn std::error::Error>> {
    info!("running container '{}'...", container.id);

    mount_container_filesystem(container)?;

    info!("making host mount namespace private...");
    mount(
        None::<&str>,
        "/",
        None::<&str>,
        MsFlags::MS_REC | MsFlags::MS_PRIVATE,
        None::<&str>,
    )?;


    info!("performing bind mount on container filesystem...");
    let rootfs_path_str = format!(
        "{}/merged",
        get_container_path(container)?
    );

    info!("cloning process...");
    let cb = Box::new(|| {
        if let Err(e) = init(rootfs_path_str.as_str()) {
            error!("unable to initialize container: {}", e);
            -1
        } else {
            0
        }
    });
    // TODO: Check if modifications are required
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

    info!("run successfull");
    Ok(())
}

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

    // TODO: Change from literals to variables
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