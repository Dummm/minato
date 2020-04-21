use std::process::{self, exit};

extern crate clap;
use clap::{crate_name, crate_version, App, Arg, SubCommand};
use log::*;

mod image;
mod image_manager;
use image::Image;
mod container;
mod container_manager;
use container::Container;


// TODO: Modularize project
// TODO: Switch overlay mounting method if root is required
fn main() {
    env_logger::init();

    let mut app = App::new(crate_name!())
        .version(crate_version!())
        .about("container runtime")//;
    // let args = &app
        .subcommand(SubCommand::with_name("pull")
            .about("pull image from docker repository")
            .arg(
                Arg::with_name("image_name")
                    .help("specify image name")
                    .short("n")
                    .long("name")
                    .takes_value(true)
                    .default_value("library/alpine")
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
            .arg(
                Arg::with_name("image_name")
                    .help("specify image name")
                    .short("i")
                    .long("image_name")
                    .takes_value(true)
                    .required(true)
                    .multiple(false),
            )
            .arg(
                Arg::with_name("cmd")
                    .short("c")
                    .long("cmd")
                    .multiple(false)
                    .default_value("/bin/sh")
                    .help("Command executed on container creation"),
            )
        )
        .subcommand(
            SubCommand::with_name("run")
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
        );

    // TODO: Parse arguments
    // let config = Config::new(args).unwrap();
    // info!("using rootfs: {}", config.root_filesystem);
    // info!("using command: {}", config.command);

    let result = match &app.clone().get_matches().subcommand() {
        ("pull",   Some(subcommand_args)) => image_manager::pull(&subcommand_args),
        ("create", Some(subcommand_args)) => container_manager::create(&subcommand_args),
        ("run",    Some(subcommand_args)) => container_manager::run(&subcommand_args),
        _ => {
            eprintln!("Unexpected arguments");
            app.print_help().unwrap();
            println!();
            exit(1);
        }
    };

    // let mut image = Image::new("library/ubuntu");
    // if let Err(e) = image_manager::pull(&mut image) {
    //     error!("image pulling unsuccessful: {}", e);
    //     process::exit(1);
    // }

    // let container = Container::new(Some(image), Some("cont"));
    // if let Err(e) = container_manager::create(&container) {
    //     error!("container creation unsuccessful: {}", e);
    //     process::exit(1);
    // };

    // if let Err(e) = container_manager::run(&container) {
    //     error!("container run unsuccessful: {}", e);
    //     process::exit(1);
    // };

}
