use std::process::exit;

extern crate structopt;
use structopt::{StructOpt, clap::crate_name};

mod cli;
mod image;
mod container;
mod utils;
mod networking;

#[derive(Debug, StructOpt)]
#[structopt(global_setting = structopt::clap::AppSettings::ColoredHelp)]
pub struct Opt {
    #[structopt(short, long)]
    daemon: bool,

    #[structopt(short = "D", long)]
    debug: bool,

    #[structopt(short, long,
        env = "RUST_LOG",
        default_value = crate_name!())]
    log_level: String,

    #[structopt(subcommand)]
    subcommand: Subcommand
}

#[derive(Debug, StructOpt)]
enum Subcommand {
    #[structopt(name = "image", about = "Manage images")]
    Image {
        #[structopt(subcommand, about = "pull|delete")]
        action: ImageAction
    },

    #[structopt(name = "container", about = "Manage containers")]
    Container {
        #[structopt(subcommand, about = "create|run|delete")]
        action: ContainerAction
    }
}

#[derive(Debug, StructOpt)]
enum ImageAction {
    #[structopt(name = "pull", about = "Pull an image from the Docker repository")]
    Pull {
        #[structopt(name = "image-id",
            about = "Image ID in Docker repository",
            short = "i", long = "image-id",
            default_value = "library/alpine:latest")]
        image_id: String,
    },

    #[structopt(name = "delete", about = "Delete an image from local storage")]
    Delete {
        #[structopt(name = "image-id",
            about = "Image ID in Docker repository",
            short = "i", long = "image-id")]
        image_id: String,
    }
}

#[derive(Debug, StructOpt)]
enum ContainerAction {
    #[structopt(name = "create", about = "Create a container")]
    Create {
        #[structopt(name = "container-name",
            about = "Container name",
            short = "c", long = "container-nane")]
        container_name: String,

        #[structopt(name = "image-id",
            about = "Container name",
            short = "i", long = "image-id",
            default_value = "library/alpine:latest")]
        image_id: String,
    },

    #[structopt(name = "run", about = "Run a container")]
    Run {
        #[structopt(name = "container-name",
            about = "Container name",
            short = "c", long = "container-nane")]
        container_name: String,
    },

    #[structopt(name = "delete", about = "Delete a container")]
    Delete {
        #[structopt(name = "container-name",
            about = "Container name",
            short = "c", long = "container-nane")]
        container_name: String,
    }
}

fn unexpected_arguments(app: clap::App) -> std::result::Result<(), Box<dyn std::error::Error>> {
    eprintln!("Unexpected arguments");
    app.clone().print_help().unwrap();
    println!();
    exit(1);
}



// TODO: Manage project
// TODO: Modularize project
// TODO: Switch overlay mounting method if root is required
// TODO: Try archivemount instead of unarchiving layers
// TODO: Work on the spec files for the config.json
// TODO: Use config.json to store container run info
// TODO: Change back names from c's to n's
// TODO: Add listing subcommands for images and containers
// TODO: Fix unwraps so it doesn't panic
// TODO: Cgroups
// TODO: Fix networking
// TODO: User namespace (uid, gid, subuid, subgid)
// TODO: Daemon

fn main() {
    env_logger::init();

    let app = Opt::clap();
    let opt = Opt::from_args();
    println!("{:?}", opt);

    let image_manager = image::ImageManager::new();
    let container_manager = container::ContainerManager::new();

    match &app.clone().get_matches().subcommand() {
        ("image", Some(subcommand)) => {
            match subcommand.subcommand() {
                ("pull", Some(subcommand_args))   => image_manager.pull_with_args(&subcommand_args),
                ("delete", Some(subcommand_args)) => image_manager.delete_with_args(&subcommand_args),
                _ => unexpected_arguments(app)
            }
        },
        ("container", Some(subcommand)) => {
            match subcommand.subcommand() {
                ("create", Some(subcommand_args)) => container_manager.create_with_args(&subcommand_args),
                ("run",    Some(subcommand_args)) => container_manager.run_with_args(&subcommand_args),
                ("delete", Some(subcommand_args)) => container_manager.delete_with_args(&subcommand_args),
                _ => unexpected_arguments(app)
            }
        },
        _ => unexpected_arguments(app)
    }.unwrap()

}
