use std::iter;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use std::fs;
use std::io::{self, prelude::*};
use std::path::Path;
use std::os::unix;
use std::process::{Command, Stdio};
use std::time;
use std::thread;
use std::ffi::CString;
use nix::mount::*;
use nix::sched::*;
use nix::unistd::{chdir, execve, mkdir, pivot_root, sethostname};
use nix::sys::{stat, wait::waitpid};
use std::env;
use std::collections::HashMap;
use nix::libc;
use nix::sys::stat::{Mode, SFlag};
use nix::unistd;

use dirs;
use log::{info, error};
use clap::ArgMatches;

use crate::image::Image;
// use crate::image_manager;
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
        // info!("image name: {}", image_name);
        let image = match Image::load(image_id).unwrap() {
            Some(image) => image,
            None        => return Ok(None)
        };
        container.image = Some(image);

        Ok(Some(container))
    }

    fn prepare_parent_filesystems(&self) -> Result<(), Box<dyn std::error::Error>> {
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
    fn start_container_process(&self, container: &Container) -> Result<(), Box<dyn std::error::Error>> {
        info!("cloning process...");

        let rootfs_path_str = format!(
            "{}/merged",
            utils::get_container_path(container)?
        );

        let cb = Box::new(|| {
            if let Err(e) = self.init(rootfs_path_str.as_str()) {
                error!("unable to initialize container: {}", e);
                self.cleanup(&container);
                -1
            } else {
                self.cleanup(&container);
                0
            }
        });
        let stack = &mut [0; 1024 * 1024];
        let mut clone_flags =
            CloneFlags::CLONE_NEWUTS |
            CloneFlags::CLONE_NEWPID |
            CloneFlags::CLONE_NEWNS |
            CloneFlags::CLONE_NEWIPC;
        // if true {
        if false {
            clone_flags |= CloneFlags::CLONE_NEWNET;
        }

        let childpid = clone(
            cb,
            stack,
            clone_flags,
            None
        )?;

        info!("child pid: {}", childpid);
        thread::sleep(time::Duration::from_millis(300));

        if false {
        // if true {
            networking::add_container_to_network(&container.id, childpid)?;
        }

        // TODO: Remove at some point
        // waitpid(childpid, None)?;

        Ok(())
    }

    // TODO: Change from hostname and cmd from literals to variables
    fn init(&self, rootfs: &str) -> Result<(), Box<dyn std::error::Error>> {
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

        // BUG: ip not installed
        // if true {
        // if false {
        //     networking::add_container_to_network(container_id)?;
        // }

        // do_exec("/bin/bash")?;
        // self.do_exec("/bin/sh")?;

        info!("uid: {} - euid: {}", unistd::Uid::current(), unistd::Uid::effective());
        info!("gid: {} - egid: {}", unistd::Gid::current(), unistd::Gid::effective());


        info!("creating devices...");
        if !Path::new("dev").exists() {
            fs::create_dir("dev")?;
        }
        info!("creating ttys");
        for i in 0..7 {
            info!("creating tty{}...", i);

            let tty_path_str = format!("/dev/tty{}", i);
            let perms =
                Mode::S_IRUSR | Mode::S_IWUSR |
                Mode::S_IRGRP | Mode::S_IWGRP |
                Mode::S_IROTH | Mode::S_IWOTH;
            if Path::new(tty_path_str.clone().as_str()).exists() {
                info!("removing /dev/tty1...");
                fs::remove_file(tty_path_str.clone())?;
            }
            stat::mknod(
                tty_path_str.clone().as_str(),
                SFlag::S_IFCHR,
                perms,
                stat::makedev(4, i)
            )?;
            unistd::chown(
                tty_path_str.as_str(),
                Some(unistd::Uid::from_raw(0)),
                Some(unistd::Gid::from_raw(0))
            )?;

        }

        self.do_exec("/sbin/init")?;
        // self.do_exec("/bin/sh")?;

        // let filtered_env : HashMap<String, String> =
        //     env::vars().filter(|&(ref k, _)|
        //         k == "TERM" || k == "TZ" || k == "LANG" || k == "PATH"
        //     ).collect();
        // // let mut filtered_env: HashMap<String, String> = HashMap::new();
        // // filtered_env.insert(String::from("PATH"), String::from("/bin:/sbin:/usr/bin:/usr/sbin:/usr/local/bin"));
        // // filtered_env.insert(String::from("TERM"), String::from("xterm-256color"));
        // // filtered_env.insert(String::from("LC_ALL"), String::from("C"));
        // let downstream_output = Command::new("/bin/sh")
        //     .stdout(Stdio::piped())
        //     .stderr(Stdio::piped())
        //     .envs(&filtered_env)
        //     .output()?
        //     .stdout;
        // println!("{}", String::from_utf8_lossy(&downstream_output));

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

    // TODO: Fix unwrap here
    #[allow(dead_code)]
    pub fn run_with_args(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let container_name = args.value_of("container-name").unwrap();
        self.run(container_name)
    }

    // TODO: Compile functions
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

        self.mount_container_filesystem(&container)?;

        let container_path_str = format!(
            "{}/merged",
            utils::get_container_path_with_str(container_name).unwrap()
        );
        let hosts = format!("{}/etc/hosts", container_path_str);
        let resolv = format!("{}/etc/resolv.conf", container_path_str);

        fs::copy("/etc/hosts", &hosts)?;
        fs::copy("/etc/resolv.conf", &resolv)?;
        info!("copied /etc/hosts and /etc/resolv.conf");

        // TODO: Add iproute2 check

        // if true {
        if false {
            // networking::create_network_namespace(&container.id)?;
            networking::create_bridge(&container.id)?;
            networking::create_veth(&container.id)?;
            networking::add_veth_to_bridge(&container.id)?;
        }

        // Unmount mounted filesystem in case of error
        if let Err(e) = self.prepare_parent_filesystems() {
            self.cleanup(&container)?;
            return Err(e);
        };
        if let Err(e) = self.start_container_process(&container) {
            self.cleanup(&container)?;
            return Err(e);
        };

        // if true {
        if false {
            networking::delete_container_from_network(&container.id)?;
            networking::remove_veth_from_bridge(&container.id)?;
            networking::delete_veth(&container.id)?;
            networking::delete_bridge(&container.id)?;
            networking::delete_network_namespace(&container.id)?;
        }

        info!("run successfull");
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