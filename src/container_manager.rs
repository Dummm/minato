use std::fs::File;
use std::path::Path;
use std::ffi::CString;
use std::os::unix::io::AsRawFd;
use std::collections::HashMap;
use nix::sys::{wait::waitpid, signal::kill, signal::Signal};
use nix::unistd::{fork, ForkResult, execve, Pid};
use nix::sched::{CloneFlags, setns};
use clap::ArgMatches;

// use nix::unistd::*;
// use nix::fcntl::{open, OFlag};
// use nix::sys::stat::Mode;

use log::{info, error};

use crate::utils;
use crate::image::Image;
use crate::container::Container;

pub struct ContainerManager<'a> {
    #[allow(dead_code)]
    container_list: Vec<&'a Container>
}
impl<'a> ContainerManager<'a> {
    /// Create a new container manager object
    pub fn new() -> ContainerManager<'a> {
        ContainerManager {
            container_list: Vec::new()
        }
    }

    #[allow(dead_code)]
    /// Create and store a new container from arguments passed to the executable
    pub fn create_with_args(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let image_name = args.value_of("image-id").unwrap();
        let container_name = args.value_of("container-name").unwrap();
        self.create(container_name, image_name)
    }
    /// Create and store a new container
    pub fn create(&self, container_name: &str, image_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("creating container '{}'...", container_name);

        let image = match Image::load(image_id)? {
            Some(image) => image,
            None        => {
                info!("image not found. exiting...");
                // info!("image not found. trying to pull image...");
                // image_manager::pull(image_id)?;
                // create(container_name, image_id)?;
                return Ok(())
            }
        };

        let container = Container::new(Some(container_name), Some(image));

        container.create()?;
        info!("created container.");
        Ok(())
    }

    #[allow(dead_code)]
    /// Run a stored container using arguments passed to the executable as parameters
    pub fn run_with_args(&self, args: &ArgMatches, daemon: bool, volume: Option<String>, host_ip: Option<String>, container_ip: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        let container_name = args.value_of("container-name").unwrap();
        self.run(container_name, daemon, volume, host_ip, container_ip)
    }
    /// Run a stored container
    pub fn run(&self, container_name: &str, daemon: bool, volume: Option<String>, host_ip: Option<String>, container_ip: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        info!("running container '{}'...", container_name);

        info!("loading container...");
        let container = match Container::load(container_name).unwrap() {
            Some(container) => container,
            None            => {
                info!("container not found. exiting...");
                return Ok(())
            }
        };

        let pid_path = format!("{}/pid", container.path);
        if Path::new(&pid_path).exists() {
            info!("container already running. exiting...");
            return Ok(())
        }

        container.run(daemon, volume, host_ip, container_ip)?;
        info!("ran container.");
        Ok(())
    }

    /// Call the setns syscall to enter a container's namespaces
    fn set_namespace(&self, fd: &str, flag: CloneFlags) -> Result<(), Box<dyn std::error::Error>> {
        // while !Path::new(fd).exists() {;}

        if !Path::new(fd).exists() {
            info!("path '{}' does not exit", fd);
            Ok(())
        } else {
            if let Err(e) = setns(File::open(fd).unwrap().as_raw_fd(), flag) {
                info!("error setting namespace {} - {:?}: {}", fd, flag, e);
                Ok(())
            } else {
                info!("ns {:?} set", flag);
                Ok(())
            }
        }
    }
    /// Execute command inside container
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
        info!("arguments: \n{:?}\n{:?}\n{:?}",
            args, envs, p);
        execve(&p, &a, &e)?;

        Ok(())
    }
    /// Open/enter a running container
    pub fn open(&self, container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("opening container...");
        // let pid = unistd::Pid::from_raw(container_pid.parse::<i32>().unwrap());

        let container_pid = match utils::get_container_pid_with_str(container_name).unwrap() {
            None => {
                info!("container isn't running or doesn't exist. exiting...");
                return Ok(());
            },
            Some(pid) => pid
        };
        info!("container pid: {}", container_pid);

        // info!("uid");
        // // let uid = getuid();
        // let uid = 0;
        // // let newuid = self.spec.process.user.uid;
        // let newuid = 0;
        // let buf = format!("{} {} 1\n", newuid, uid);
        // let fd = open("/proc/self/uid_map", OFlag::O_WRONLY, Mode::empty())?;
        // info!("writing 'uid_map'");
        // write(fd, buf.as_bytes())?;
        // close(fd)?;

        // // let fd = open("/proc/self/setgroups", OFlag::O_WRONLY, Mode::empty())?;
        // // info!("writing 'deny' to setgroups");
        // // write(fd, "deny".as_bytes())?;
        // // close(fd)?;

        // info!("gid");
        // // let gid = getgid();
        // let gid = 0;
        // // let newgid = self.spec.process.user.gid;
        // let newgid = 0;
        // let buf = format!("{} {} 1\n", newgid, gid);
        // let fd = open("/proc/self/gid_map", OFlag::O_WRONLY, Mode::empty())?;
        // info!("writing 'gid_map'");
        // write(fd, buf.as_bytes())?;
        // close(fd)?;

        let mut namespaces = HashMap::new();
        namespaces.insert(CloneFlags::CLONE_NEWIPC, "ipc");
        namespaces.insert(CloneFlags::CLONE_NEWUTS, "uts");
        namespaces.insert(CloneFlags::CLONE_NEWNET, "net");
        namespaces.insert(CloneFlags::CLONE_NEWPID, "pid");
        namespaces.insert(CloneFlags::CLONE_NEWNS, "mnt");
        namespaces.insert(CloneFlags::CLONE_NEWCGROUP, "cgroup");
        namespaces.insert(CloneFlags::CLONE_NEWUSER, "user");

        let pid_path = format!("/proc/{}/ns", container_pid);
        info!("setting namespaces...");
        for namespace in namespaces {
            let ns_path = format!("{}/{}", pid_path, namespace.1);
            self.set_namespace(ns_path.as_str(), namespace.0)?;
        }

        let result = match fork() {
            Ok(ForkResult::Parent { child, .. }) => {
                waitpid(child, None)?;
                Ok(())
            }
            Ok(ForkResult::Child) => {
                self.do_exec("/bin/sh")
            }
            Err(e) => {
                info!("fork failed: {}", e);
                Ok(())
            }
        };

        info!("opened container.");
        result
    }

    /// Stop a running container
    pub fn stop(&self, container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("stopping container...");

        let pid = match utils::get_container_pid_with_str(container_name).unwrap() {
            None => {
                info!("container isn't running or doesn't exist. exiting...");
                return Ok(());
            },
            Some(pid) => pid
        };
        info!("container pid: {}", pid);

        info!("killing process...");
        let pid_int: i32 = pid.parse()?;
        kill(Pid::from_raw(pid_int), Signal::SIGTERM)?;

        info!("stopped container.");
        Ok(())
    }

    /// List all stored containers
    pub fn list(&self) -> Result<(), Box<dyn std::error::Error>> {
        // let home = match dirs::home_dir() {
        //     Some(path) => path,
        //     None       => return Err("error getting home directory".into())
        // };
        // let containers_path = format!("{}/.minato/containers", home.display());
        let containers_path = format!("/var/lib/minato/containers");
        let containers_path = Path::new(&containers_path);
        if !containers_path.exists() {
            error!("containers path not found. exiting...");
            return Ok(());
        };

        // debug!("{}", containers_path.display());
        let containers = containers_path.read_dir()?;
        let containers = containers
            .map(|dir|
                format!("{}",
                    dir.unwrap()
                    .path()
                    .file_name().unwrap()
                    .to_str().unwrap()))
            .collect::<Vec<String>>()
            .clone();

        // debug!("{:?}", containers);
        println!(
            "{:10} {:30} {:30} {}",
            "pid", "id", "image", "path");
        for c in containers {
            let container = match Container::load(c.as_str()) {
                Ok(c) => c,
                Err(e) => {
                    println!("error: {}", e);
                    continue
                }
            };
            match container {
                Some(cont) => {
                    let image = match &cont.image {
                        Some(img) => img.id.clone(),
                        None      => String::from("-")
                    };
                    let pid = match utils::get_container_pid(&cont)? {
                        Some(pid) => pid,
                        None => String::from("-")
                    };

                    println!(
                        "{:10} {:30} {:30} {}",
                        pid, cont.id, image, cont.path);
                },
                None => continue
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    /// Delete a stored container using arguments passed to the executable as parameters
    pub fn delete_with_args(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let container_name = args.value_of("container-name").unwrap();
        self.delete(container_name)
    }
    /// Delete a stored container
    pub fn delete(&self, container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("deleting container...");
        let container = Container::new(Some(container_name), None);

        container.delete()?;
        info!("deleted container.");
        Ok(())
    }
}
