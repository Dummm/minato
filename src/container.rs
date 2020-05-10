use std::iter;
use std::fs;
use std::io::{self, prelude::*};
use std::path::Path;
use std::os::unix;
use std::process::Command;
use std::time;
use std::thread;
use std::ffi::CString;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use nix::mount::*;
use nix::sched::*;
use nix::unistd::{chdir, execve, mkdir, pivot_root, sethostname};
use nix::sys::stat::{self, Mode, SFlag};
use nix::unistd;
use nix::unistd::{fork, ForkResult};
use nix::sys::wait::waitpid;
use nix::fcntl::{open, OFlag};

use dirs;
use log::{info, error};
use clap::ArgMatches;

use crate::image::Image;
use crate::utils;
use crate::networking;



// #[derive(Debug, PartialEq)]
// pub enum State {
//     Creating,
//     Created(u32),
//     Running(u32),
//     Stopped,
// }

// TODO: Save/load container structures in jsons (serde_json)
pub struct Container {
    pub id: String,
    pub image: Option<Image>,
    // pub state: State,
}
// TODO: Add methods for container paths
impl Container {
    pub fn new(container_id: Option<&str>, image: Option<Image>) -> Container {
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
            // state: State::Stopped,
        }
    }
}


pub struct ContainerManager<'a> {
    container_list: Vec<&'a Container>
}
impl<'a> ContainerManager<'a> {
    pub fn new() -> ContainerManager<'a> {
        ContainerManager {
            container_list: Vec::new()
        }
    }


    fn create_directory_structure(&self, container: &Container) -> Result<(), Box<dyn std::error::Error>> {
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
    fn generate_config_json(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    #[allow(dead_code)]
    pub fn create_with_args(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let image_name = args.value_of("image-id").unwrap();
        let container_name = args.value_of("container-name").unwrap();
        self.create(container_name, image_name)
    }

    pub fn create(&self, container_name: &str, image_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let image = match Image::load(image_id)? {
            Some(image) => image,
            None        => {
                // TODO: Add verification
                info!("image not found. exiting...");
                // info!("image not found. trying to pull image...");
                // image_manager::pull(image_id)?;
                // create(container_name, image_id)?;
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

        self.generate_config_json()?;
        self.create_directory_structure(&container)?;

        Ok(())
    }



    // TODO: Find a better way to find image
    pub fn load_container(&self, container_name: &str) -> Result<Option<Container>, Box<dyn std::error::Error>> {
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
        let image = match Image::load(image_id).unwrap() {
            Some(image) => image,
            None        => return Ok(None)
        };
        container.image = Some(image);

        Ok(Some(container))
    }

    fn mount_container_filesystem(&self, container: &Container)  -> Result<(), Box<dyn std::error::Error>> {
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

        info!("mount output: {}", output.status);
        io::stdout().write_all(&output.stdout).unwrap();
        io::stderr().write_all(&output.stderr).unwrap();

        Ok(())
    }

    fn do_exec(&self, cmd: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("preparing command execution...");

        let args = &[Path::new(cmd).file_stem().unwrap().to_str().unwrap()];
        let envs = &[
            "PATH=/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin",
            "TERM=xterm-256color",
            "LC_ALL=C"
        ];
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

    fn unmount_container_filesystem(&self, container: &Container) -> Result<(), Box<dyn std::error::Error>> {
        let container_path_str = utils::get_container_path(container)?;
        let merged = format!("{}/merged", container_path_str);
        umount2(merged.as_str(), MntFlags::MNT_DETACH)?;

        Ok(())
    }

    fn cleanup(&self, container: &Container) -> Result<(), Box<dyn std::error::Error>> {
        info!("cleaning up container...");

        self.unmount_container_filesystem(container)?;

        Ok(())
    }

    pub fn prepare_container_directories(&self, container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("preparing container directories...");

        let rootfs_path_str = format!(
            "{}/merged",
            utils::get_container_path_with_str(container_name)?
        );
        let rootfs = rootfs_path_str.as_str();

        utils::prepare_directory(
            rootfs,
            "put_old",
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IXUSR |
            Mode::S_IRGRP |                 Mode::S_IXGRP |
            Mode::S_IROTH |                 Mode::S_IXOTH
        )?;

        utils::prepare_directory(
            rootfs,
            "dev",
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IXUSR |
            Mode::S_IRGRP |                 Mode::S_IXGRP |
            Mode::S_IROTH |                 Mode::S_IXOTH
        )?;

        utils::prepare_directory(
            rootfs,
            "sys",
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IXUSR |
            Mode::S_IRGRP |                 Mode::S_IXGRP |
            Mode::S_IROTH |                 Mode::S_IXOTH
        )?;

        utils::prepare_directory(
            rootfs,
            "proc",
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IXUSR |
            Mode::S_IRGRP |                 Mode::S_IXGRP |
            Mode::S_IROTH |                 Mode::S_IXOTH
        )?;
        utils::prepare_directory(
            rootfs,
            "old_proc",
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IXUSR |
            Mode::S_IRGRP |                 Mode::S_IXGRP |
            Mode::S_IROTH |                 Mode::S_IXOTH
        )?;

        Ok(())
    }

    pub fn prepare_container_ids(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("preparing container ids...");

        info!("uid: {} - euid: {}", unistd::Uid::current(), unistd::Uid::effective());
        info!("gid: {} - egid: {}", unistd::Gid::current(), unistd::Gid::effective());

        // let newuid = 3333;
        // let newuid = 1000;
        let newuid = 0;
        // let uid = unistd::getuid();
        let uid = 0;
        let buf = format!("{} {} 1\n", newuid, uid);
        let fd = open("/proc/self/uid_map", OFlag::O_WRONLY, Mode::empty())?;
        info!("writing 'uid_map'");
        unistd::write(fd, buf.as_bytes())?;
        unistd::close(fd)?;

        let fd = open("/proc/self/setgroups", OFlag::O_WRONLY, Mode::empty())?;
        info!("writing 'deny' to setgroups");
        unistd::write(fd, "deny".as_bytes())?;
        unistd::close(fd)?;

        // let newgid = 3333;
        // let newgid = 1000;
        let newgid = 0;
        // let gid = unistd::getgid();
        let gid = 0;
        let buf = format!("{} {} 1\n", newgid, gid);
        let fd = open("/proc/self/gid_map", OFlag::O_WRONLY, Mode::empty())?;
        info!("writing 'gid_map' (could fail)");
        unistd::write(fd, buf.as_bytes())?;
        unistd::close(fd)?;

        // info!("setting ids");
        // let root_uid = unistd::Uid::from_raw(newuid);
        // let root_gid = unistd::Gid::from_raw(newgid);
        // unistd::setresuid(root_uid, root_uid, root_uid)?;
        // unistd::setresgid(root_gid, root_gid, root_gid)?;

        Ok(())
    }

    // TODO: Fix unwrap here
    #[allow(dead_code)]
    pub fn run_with_args(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let container_name = args.value_of("container-name").unwrap();
        self.run(container_name)
    }

    pub fn run(&self, container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("running container '{}'...", container_name);

        info!("loading container...");
        let container = match self.load_container(container_name).unwrap() {
            Some(container) => container,
            None            => {
                info!("container not found. exiting...");
                return Ok(())
            }
        };

        info!("mounting overlayfs...");
        self.mount_container_filesystem(&container)?;

        let clone_flags =
            CloneFlags::CLONE_NEWPID |
            CloneFlags::CLONE_NEWNS |
            CloneFlags::CLONE_NEWUTS |
            CloneFlags::CLONE_NEWIPC |
            CloneFlags::CLONE_NEWUSER;
        info!("unsharing...");
        unshare(clone_flags)?;

        info!("making root private...");
        mount(
            None::<&str>,
            "/",
            None::<&str>,
            MsFlags::MS_REC | MsFlags::MS_PRIVATE,
            None::<&str>,
        )?;

        let rootfs_path_str = format!(
            "{}/merged",
            utils::get_container_path_with_str(container_name)?
        );
        let rootfs = rootfs_path_str.as_str();
        info!("mounting rootfs...");
        mount(
            Some(rootfs),
            rootfs,
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_NOSUID,
            None::<&str>,
        )?;

        info!("chdir to rootfs ({})...", rootfs);
        chdir(rootfs)?;

        self.prepare_container_directories(container_name)?;

        info!("mounting proc to old_proc...");
        mount(
            Some("/proc"),
            "old_proc",
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_REC,
            None::<&str>,
        )?;

        info!("mounting sys...");
        mount(
            Some("/sys"),
            "sys",
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_REC,
            None::<&str>,
        )?;

        info!("mounting dev...");
        mount(
            Some("/dev"),
            "dev",
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_REC,
            None::<&str>,
        )?;

        self.prepare_container_ids()?;

        info!("pivoting root...");
        pivot_root(".", "put_old")?;

        info!("unmounting pivot auxiliary folder...");
        umount2("/put_old", MntFlags::MNT_DETACH)?;
        if Path::new("/put_old").exists() {
            info!("removing auxiliary folder...");
            std::fs::remove_dir_all("/put_old")?;
        }

        match unistd::fork() {
            Ok(ForkResult::Child) => {
                info!("child pid: {}", unistd::getpid());

                info!("mounting proc...");
                mount(
                    Some("/proc"),
                    "/proc",
                    Some("proc"),
                    MsFlags::MS_NOSUID,
                    None::<&str>,
                )?;

                info!("unmounting proc folder...");
                umount2("/old_proc", MntFlags::MNT_DETACH)?;
                info!("removing proc auxiliary folder...");
                if Path::new("/old_proc").exists() {
                    info!("removing old 'old_proc' folder...");
                    std::fs::remove_dir_all("/old_proc")?;
                }

                info!("mounting root...");
                mount(
                    Some("/"),
                    "/",
                    None::<&str>,
                    MsFlags::MS_BIND | MsFlags::MS_RDONLY | MsFlags::MS_NOSUID, // | MsFlags::MS_REMOUNT,
                    None::<&str>,
                )?;

                sethostname("test")?;
                self.do_exec("/bin/sh")?;

                info!("exiting...");
                std::process::exit(0);
            }
            Ok(ForkResult::Parent { child, .. }) => {
                info!("aici");
                waitpid(child, None)?;
            }

            Err(_) => {}
        };

        info!("aici aici");
        Ok(())

    }

    #[allow(dead_code)]
    pub fn delete_with_args(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let container_name = args.value_of("container-name").unwrap();
        self.delete(container_name)
    }

    // TODO: Add contianer state check
    pub fn delete(&self, container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
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
}