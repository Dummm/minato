use std::fs;
use std::io::prelude::*;
use std::io;
use std::iter;
use std::path::Path;
use std::os::unix;
use std::process::Command;

use dirs;

// for generate container.id
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use log::info;

use super::image::Image;
// use super::mounts;
// use super::pids::Pidfile;
// use super::process::Process;
use super::image_manager;

pub struct Container {
    pub id: String,
    pub image: Option<Image>,
    pub state: State,
}

#[derive(Debug, PartialEq)]
pub enum State {
    Creating,
    Created(u32),
    Running(u32),
    Stopped,
}

impl Container {
    pub fn new(image: Option<Image>, container_id: Option<&str>) -> Container {
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
            state: State::Stopped,
        }
    }
}


pub fn create_directory_structure(container: &Container) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating container directory structure...");

    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };

    let container_path_str = format!(
        "{}/.minato/containers/{}",
        home.display(), container.id
    );
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
            image_manager::get_image_path(&container.image.as_ref().unwrap()).unwrap(),
            container_lower_path
        )?;
    }

    Ok(())
}

// TODO: Generate config.json file
fn generate_config_json() -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

pub fn create(container: &Container) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating container '{}'...", container.id);

    generate_config_json()?;
    create_directory_structure(container)?;

    Ok(())
}

fn mount_container_filesystem(container: &Container)  -> Result<(), Box<dyn std::error::Error>> {
    info!("mounting container filesystem...");

    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };
    let container_path_str = format!(
        "{}/.minato/containers/{}",
        home.display(), container.id
    );
    let container_path = Path::new(container_path_str.as_str());

    // TODO: Fix this mess
    let mut lowerdir_arg = "lowerdir=".to_string();
    let subdirectories = container_path.join("lower").read_dir()?;
    let subdirectories_vector = subdirectories
        .map(|dir| format!("{}", dir.unwrap().path().display()))
        // .map(|dir| format!("lower/{:?}", dir.unwrap().file_name()))
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

    info!("mount output {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    Ok(())
}

pub fn run(container: &Container) -> Result<(), Box<dyn std::error::Error>> {
    info!("running container '{}'...", container.id);

    mount_container_filesystem(container)?;

    Ok(())
}

// pub fn prepare(, process: &Process) {
//     // specify Image name
//     if let Some(image) = &mut self.image {
//         self.state = State::Creating;
//         image.pull().expect("Failed to cromwell pull");
//         image
//             .build_from_tar(&process.cwd)
//             .expect("Failed build image from fsLayer");

//         let c_hosts = format!("{}/etc/hosts", process.cwd);
//         let c_resolv = format!("{}/etc/resolv.conf", process.cwd);

//         fs::copy("/etc/hosts", &c_hosts).expect("Failed copy /etc/hosts");
//         info!("[Host] Copied /etc/hosts to {}", c_hosts);

//         fs::copy("/etc/resolv.conf", &c_resolv).expect("Failed copy /etc/resolv.conf");
//         info!("[Host] Copied /etc/resolv.conf {}", c_resolv);
//     }

//     // nochdir, close tty
//     if process.become_daemon {
//         daemon(true, false).expect("cannot become daemon");
//     }

//     unshare(
//         CloneFlags::CLONE_NEWPID
//             | CloneFlags::CLONE_NEWUTS
//             | CloneFlags::CLONE_NEWNS
//             | CloneFlags::CLONE_NEWUSER,
//     )
//     .expect("Can not unshare(2).");

//     self.guid_map(&process)
//         .expect("Failed to write /proc/self/gid_map|uid_map");
//     self.state = State::Created;
// }

// pub fn run(&mut self, process: &Process) {
//     match fork() {
//         Ok(ForkResult::Parent { child, .. }) => {
//             info!("[Host] PID: {}", getpid());
//             info!("[Container] PID: {}", child);

//             let home = home_dir().expect("Could not get your home_dir");
//             let home = home.to_str().expect("Could not PathBuf to str");
//             let pids_path = format!("{}/.cromwell/pids", home);
//             fs::create_dir_all(&pids_path).expect("failed mkdir pids");

//             let pidfile_path = format!("{}/{}.pid", pids_path, self.id);
//             let pidfile_path = Path::new(&pidfile_path);

//             Pidfile::create(&pidfile_path, child).expect("Failed to create pidfile");

//             match waitpid(child, None).expect("waitpid faild") {
//                 WaitStatus::Exited(_, _) => {
//                     Pidfile::delete(&pidfile_path).expect("Failed to remove pidfile");
//                     self.state = State::Stopped;
//                 }
//                 WaitStatus::Signaled(_, _, _) => {}
//                 _ => eprintln!("Unexpected exit."),
//             }
//         }
//         Ok(ForkResult::Child) => {
//             self.state = State::Running;
//             chroot(Path::new(&process.cwd)).expect("chroot failed.");
//             chdir("/").expect("cd / failed.");

//             sethostname(&self.id).expect("Could not set hostname");
//             fs::create_dir_all("proc").unwrap_or_else(|why| {
//                 eprintln!("{:?}", why.kind());
//             });

//             info!("[Container] Mount procfs ... ");
//             mounts::mount_proc().expect("mount procfs failed");

//             execve(&process.cmd[0], &process.cmd, &process.env).expect("execution failed.");
//         }
//         Err(e) => panic!("Fork failed: {}", e),
//     }
// }

// pub fn delete(&self, process: &Process) -> std::io::Result<()> {
//     fs::remove_dir_all(&process.cwd)
// }


