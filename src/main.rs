use std::process::exit;

extern crate clap;
use clap::{App, Arg, SubCommand, crate_name, crate_version};
use log::*;

mod image;
mod image_manager;
mod container;
mod container_manager;
mod utils;


// TODO: Modularize project
// TODO: Switch overlay mounting method if root is required
// TODO: Try archivemount instead of unarchiving layers
// TODO: Work on the spec files for the config.json
fn main() {
    env_logger::init();

    let mut app = App::new(crate_name!())
        .version(crate_version!())
        .about("container runtime")
        .subcommand(SubCommand::with_name("pull")
            .about("pull image from docker repository")
            .arg(
                Arg::with_name("image_name")
                    .help("specify image name")
                    .short("n")
                    .long("image_name")
                    .takes_value(true)
                    .default_value("library/alpine:latest")
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
                    .default_value("library/alpine:latest")
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


    info!("{} {}", crate_name!(), crate_version!());
    info!("starting...");

    match &app.clone().get_matches().subcommand() {
        ("pull",   Some(subcommand_args)) => image_manager::pull_with_args(&subcommand_args),
        ("create", Some(subcommand_args)) => container_manager::create_with_args(&subcommand_args),
        ("run",    Some(subcommand_args)) => container_manager::run_with_args(&subcommand_args),
        _ => {
            eprintln!("Unexpected arguments");
            app.print_help().unwrap();
            println!();
            exit(1);
        }
    }.unwrap()
}
