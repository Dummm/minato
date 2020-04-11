extern crate clap;
use clap::{crate_name, crate_version, App, Arg, SubCommand};
// extern crate nix;
// extern crate pretty_env_logger;

// extern crate ipaddress;
// extern crate num;
// extern crate rand;

use log::*;
// use log::Level;

// use nix::sched::*;
// use nix::{Error, Result};
// use nix::sched::clone;
// use nix::unistd::{chdir, execve, mkdir, pivot_root, sethostname};
// use nix::mount::*;
// use std::ffi::CString;

// use std::time;
// use std::thread;

// use nix::sys::stat;
// use nix::sys::wait::waitpid;
// use minato::Config;
// use minato::Process;

mod image;
mod image_manager;
use image::Image;

mod container_manager;

use std::process;
// use std::ffi::CString;

// TODO: Modularize project
fn main() {
    env_logger::init();

    let args = App::new(crate_name!())
        .version(crate_version!())
        .about("container runtime")
        .subcommand(SubCommand::with_name("image")
            .about("manage images")
            .subcommand(SubCommand::with_name("pull")
                .about("pull image from docker repository")
                .arg(
                    Arg::with_name("image_name")
                        .help("specify image name")
                        .short("n")
                        .long("name")
                        .takes_value(true)
                        .required(true)
                        .multiple(false),
                )
            )
        )
        .subcommand(SubCommand::with_name("container")
            .about("manage containers")
            .subcommand(SubCommand::with_name("run")
                .about("run container")
                .arg(
                    Arg::with_name("container_name")
                        .help("specify container name")
                        .short("n")
                        .long("name")
                        .takes_value(true)
                        .required(true)
                        .multiple(false),
                )
            )
            .subcommand(SubCommand::with_name("create")
                .about("create container")
                .arg(
                    Arg::with_name("container_name")
                        .help("specify container name")
                        .short("n")
                        .long("name")
                        .takes_value(true)
                        .required(true)
                        .multiple(false),
                )
                // .arg(
                //     Arg::with_name("rootfs")
                //         .required(true)
                //         .short("r")
                //         .long("rootfs")
                //         .multiple(false)
                //         .takes_value(true)
                //         .help("Path of the container root file system"),
                // )
                // .arg(
                //     Arg::with_name("cmd")
                //         .short("c")
                //         .long("cmd")
                //         .multiple(false)
                //         .default_value("/bin/sh")
                //         .help("Command executed on container creation"),
                // )
            )
        );

    // TODO: Parse arguments
    // let config = Config::new(args).unwrap();
    // info!("using rootfs: {}", config.root_filesystem);
    // info!("using command: {}", config.command);

    let mut image = Image::new("library/ubuntu");
    if let Err(e) = image_manager::pull(&mut image) {
        error!("image pulling unsuccessful: {}", e);
        process::exit(1);
    }

    let mut container = container_manager::Container::new(Some(image), Some("cont"));
    if let Err(e) = container_manager::create(&container) {
        error!("container creation unsuccessful: {}", e);
        process::exit(1);
    };

    if let Err(e) = container_manager::run(&container) {
        error!("container run unsuccessful: {}", e);
        process::exit(1);
    };

    // let process = Process::new(
    //     vec_cstr![command],
    //     format!("{}/{}/rootfs", container_dir, container.id),
    //     become_daemon,
    //     // Example environment
    //     vec_cstr![
    //         "PATH=/bin/:/usr/bin/:/usr/local/bin:/sbin:/usr/sbin",
    //         "TERM=xterm-256color",
    //         "LC_ALL=C"
    //     ],
    // );

    // if sub_m.is_present("del") {
    //     container
    //         .delete(&process)
    //         .expect("Failed to remove container: ");
    // }

    // container.prepare(&process);
    // container.run(&process);
}
