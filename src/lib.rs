extern crate log;
extern crate clap;

use log::*;
use clap::App;
use std::path::Path;


pub struct Config {
    pub root_filesystem: String,
    pub command: String,
}

impl Config {
    pub fn new(args: App) -> Result<Config, String> {
        let matches = args.get_matches();

        let rootfs = match matches.value_of("rootfs") {
            Some(path) => path,
            None       => return Err("invalid root path".to_string())
        };
        if !Path::new(&rootfs).exists() {
            return Err(format!("root path '{}' doesn't exist", rootfs));
        }
        info!("using root path: {}", rootfs);

        let cmd = match matches.value_of("cmd") {
            Some(cmd) => cmd,
            None      => return Err("invalid command".to_string())
        };
        info!("using command: {}", cmd);

        Ok(Config {
            root_filesystem: rootfs.to_string(),
            command: cmd.to_string()
        })
    }
}

use std::ffi::CString;

use nix::unistd::{getgid, getuid, Gid, Uid};

pub struct Process {
    pub cmd: Vec<CString>,
    pub host_uid: Uid,
    pub host_gid: Gid,
    pub cwd: String,
    pub become_daemon: bool,
    pub env: Vec<CString>,
}

impl Process {
    pub fn new(cmd: Vec<CString>, cwd: String, become_daemon: bool, env: Vec<CString>) -> Self {
        Process {
            cwd,
            env,
            become_daemon,
            cmd,
            host_uid: getuid(),
            host_gid: getgid(),
        }
    }
}
