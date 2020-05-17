use std::fs::File;
use std::path::Path;
use std::ffi::CString;
use std::os::unix::io::AsRawFd;
use std::collections::HashMap;
use nix::sys::{wait::waitpid, signal::kill, signal::Signal};
use nix::unistd::{fork, ForkResult, execve, Pid};
use nix::sched::{CloneFlags, setns};
use clap::ArgMatches;

use log::info;

use crate::utils;
use crate::image::Image;
use crate::container::Container;

pub struct ContainerManager<'a> {
    #[allow(dead_code)]
    container_list: Vec<&'a Container>
}
impl<'a> ContainerManager<'a> {
    pub fn new() -> ContainerManager<'a> {
        ContainerManager {
            container_list: Vec::new()
        }
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

        container.create()
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
        let container = match Container::load(container_name).unwrap() {
            Some(container) => container,
            None            => {
                info!("container not found. exiting...");
                return Ok(())
            }
        };

        container.run()
    }

    fn set_namespace(&self, fd: &str, flag: CloneFlags) -> Result<(), Box<dyn std::error::Error>> {
        if !Path::new(fd).exists() {
            Ok(())
        } else {
            if let Err(e) = setns(File::open(fd).unwrap().as_raw_fd(), flag) {
                info!("error setting namespace {} - {:?}: {}", fd, flag, e);
                Ok(())
            } else {
                Ok(())
            }
        }
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
        info!("arguments: \n{:?}\n{:?}\n{:?}",
            args, envs, p);
        execve(&p, &a, &e)?;

        Ok(())
    }
    pub fn open(&self, container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("opening container...");
        // let pid = unistd::Pid::from_raw(container_pid.parse::<i32>().unwrap());

        let container_pid = match utils::get_container_pid_with_str(container_name).unwrap() {
            None => {
                info!("container isn't running or doesn't exist. skipping...");
                return Ok(());
            },
            Some(pid) => pid
        };
        info!("container pid: {}", container_pid);

        let mut namespaces = HashMap::new();
        namespaces.insert(CloneFlags::CLONE_NEWIPC, "ipc");
        namespaces.insert(CloneFlags::CLONE_NEWUTS, "uts");
        namespaces.insert(CloneFlags::CLONE_NEWNET, "net");
        namespaces.insert(CloneFlags::CLONE_NEWPID, "pid");
        namespaces.insert(CloneFlags::CLONE_NEWNS, "mnt");
        namespaces.insert(CloneFlags::CLONE_NEWCGROUP, "cgroup");
        namespaces.insert(CloneFlags::CLONE_NEWUSER, "user");

        let pid_path = format!("/proc/{}/ns", container_pid);
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

        info!("container opened successfully...");
        result
    }

    pub fn stop(&self, container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("stopping container...");

        let pid = match utils::get_container_pid_with_str(container_name).unwrap() {
            None => {
                info!("container isn't running or doesn't exist. skipping...");
                return Ok(());
            },
            Some(pid) => pid
        };
        info!("container pid: {}", pid);

        info!("killing process...");
        let pid_int: i32 = pid.parse()?;
        kill(Pid::from_raw(pid_int), Signal::SIGTERM)?;

        info!("container stopped successfully");
        Ok(())
    }

    #[allow(dead_code)]
    pub fn delete_with_args(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let container_name = args.value_of("container-name").unwrap();
        self.delete(container_name)
    }
    // TODO: Add contianer state check
    pub fn delete(&self, container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let container = Container::new(Some(container_name), None);

        container.delete()
    }
}
