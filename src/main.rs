use std::process::exit;

extern crate clap;
use clap::{App, Arg, SubCommand, crate_name, crate_version};

mod image;
mod image_manager;
mod container;
mod container_manager;
mod utils;
mod networking;


// TODO: Modularize project
// TODO: Switch overlay mounting method if root is required
// TODO: Try archivemount instead of unarchiving layers
// TODO: Work on the spec files for the config.json
// TODO: Use config.json to store container states?
// TODO: Change back names from c's to n's
// TODO: Add listing subcommands for images and containers
// TODO: Fix unwraps so it doesn't panic
// TODO: Cgroups
// TODO: Fix networking
fn main() {
    env_logger::init();

    let mut app = App::new(crate_name!())
        .version(crate_version!())
        .about("container runtime")
        .subcommand(SubCommand::with_name("pull-image")
            .about("pull image from docker repository")
            .arg(
                Arg::with_name("image-id")
                    .help("specify image name")
                    .short("i")
                    .long("image-id")
                    .takes_value(true)
                    .default_value("library/alpine:latest")
                    .required(true)
                    .multiple(false),
            )
        )
        .subcommand(SubCommand::with_name("delete-image")
            .about("delete image")
            .arg(
                Arg::with_name("image-id")
                    .help("specify image name")
                    .short("i")
                    .long("image-id")
                    .takes_value(true)
                    .required(true)
                    .multiple(false),
            )
        )
        .subcommand(SubCommand::with_name("create-container")
            .about("create container")
            .arg(
                Arg::with_name("container-name")
                    .help("specify container name")
                    .short("c")
                    .long("container-name")
                    .takes_value(true)
                    .required(true)
                    .multiple(false),
            )
            .arg(
                Arg::with_name("image-id")
                    .help("specify image name")
                    .short("i")
                    .long("image-id")
                    .takes_value(true)
                    .default_value("library/alpine:latest")
                    .required(true)
                    .multiple(false),
            )
            // .arg(
            //     Arg::with_name("cmd")
            //         .short("c")
            //         .long("cmd")
            //         .multiple(false)
            //         .default_value("/bin/sh")
            //         .help("Command executed on container creation"),
            // )
        )
        .subcommand(SubCommand::with_name("run-container")
            .about("run container")
            .arg(
                Arg::with_name("container-name")
                    .help("specify container name")
                    .short("c")
                    .long("container-name")
                    .takes_value(true)
                    .required(true)
                    .multiple(false),
            )
        )
        .subcommand(SubCommand::with_name("delete-container")
            .about("delete container")
            .arg(
                Arg::with_name("container-name")
                    .help("specify container name")
                    .short("c")
                    .long("container-name")
                    .takes_value(true)
                    .required(true)
                    .multiple(false),
            )
        );

    match &app.clone().get_matches().subcommand() {
        ("pull-image",       Some(subcommand_args)) => image_manager::pull_with_args(&subcommand_args),
        ("delete-image",     Some(subcommand_args)) => image_manager::delete_with_args(&subcommand_args),
        ("create-container", Some(subcommand_args)) => container_manager::create_with_args(&subcommand_args),
        ("run-container",    Some(subcommand_args)) => container_manager::run_with_args(&subcommand_args),
        ("delete-container", Some(subcommand_args)) => container_manager::delete_with_args(&subcommand_args),
        _ => {
            eprintln!("Unexpected arguments");
            app.print_help().unwrap();
            println!();
            exit(1);
        }
    }.unwrap()

}