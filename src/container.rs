use std::iter;
use std::fs;
use std::path::Path;
use std::os::unix;
use std::ffi::CString;
use nix::mount::{mount, MntFlags, MsFlags, umount, umount2};
use nix::sched::{CloneFlags, unshare};
use nix::sys::stat::{Mode};
use nix::sys::wait::waitpid;
use nix::unistd::*;
use nix::fcntl::{open, OFlag};
#[allow(unused_imports)]
use rand::{distributions::Alphanumeric, thread_rng, Rng};

use dirs;
use log::info;

use crate::image::Image;
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
    pub path: String
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
        let path = utils::get_container_path_with_str(id.as_str()).unwrap();
        // let path = String::from();

        Container {
            id,
            image,
            path
            // state: State::Stopped,
        }
    }

    // TODO: Generate config.json file
    fn generate_config_json(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
    fn create_directory_structure(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("creating container directory structure...");

        let container_path_str = utils::get_container_path(&self)?;
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
                utils::get_image_path(&self.image.as_ref().unwrap()).unwrap(),
                container_lower_path
            )?;
        }

        Ok(())
    }
    pub fn create(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("creating container '{}'...", &self.id);

        if Path::new(&self.path).exists() {
            info!("container exists. skipping creation...");
            return Ok(())
        }

        self.generate_config_json()?;
        self.create_directory_structure()?;

        info!("container created successfully");
        Ok(())
    }

    // TODO: Find a better way to find image
    pub fn load(container_name: &str) -> Result<Option<Container>, Box<dyn std::error::Error>> {
        let mut container = Container::new(Some(container_name), None);

        let container_path = Path::new(&container.path);
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

        let image_id = container_image_path
            .strip_prefix(images_path)
            .unwrap()
            .to_str()
            .unwrap();
        let image = match Image::load(image_id).unwrap() {
            Some(image) => image,
            None        => return Ok(None)
        };
        container.image = Some(image);

        Ok(Some(container))
    }

    fn mount_container_filesystem(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("mounting container filesystem...");

        let container_path = Path::new(&self.path);

        let subdirectories = container_path.join("lower")
            .read_dir().unwrap()
            .map(|dir|
                format!("{}", dir.unwrap().path().display()))
            .collect::<Vec<String>>()
            .join(":");
        let lowerdir_arg  = format!("lowerdir={}",       subdirectories);
        let upperdir_arg  = format!("upperdir={}/upper", &self.path);
        let workdir_arg   = format!("workdir={}/work",   &self.path);
        let mergeddir_arg = format!("{}/merged",         &self.path);
        // let full_arg = format!("{},{},{},index=on",
        let full_arg = format!("{},{},{}",
            lowerdir_arg, upperdir_arg, workdir_arg
        );
        info!("mount arguments: \n{}\n{}\n{}\n{}",
            lowerdir_arg, upperdir_arg, workdir_arg, mergeddir_arg);

        // let output = Command::new("./fuse-overlayfs/fuse-overlayfs")
        //     .arg(full_arg)
        //     .arg(mergeddir_arg)
        //     .output()
        //     .unwrap();

        // info!("mount output: {}", output.status);
        // io::stdout().write_all(&output.stdout).unwrap();
        // io::stderr().write_all(&output.stderr).unwrap();
        mount(
            Some("overlay"),
            mergeddir_arg.as_str(),
            Some("overlay"),
            MsFlags::empty(),
            Some(full_arg.as_str())
        )?;

        info!("container filesystem mounted successfully...");
        Ok(())
    }
    fn prepare_container_mountpoint(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("preparing container mountpoint...");

        let clone_flags =
            CloneFlags::CLONE_NEWPID |
            CloneFlags::CLONE_NEWNS |
            CloneFlags::CLONE_NEWUTS |
            CloneFlags::CLONE_NEWIPC |
            CloneFlags::CLONE_NEWUSER;
        info!("unsharing parent namespaces...");
        unshare(clone_flags)?;

        info!("making parent root private...");
        mount(
            None::<&str>,
            "/",
            None::<&str>,
            MsFlags::MS_REC | MsFlags::MS_PRIVATE,
            None::<&str>,
        )?;

        let rootfs_path_str = format!(
            "{}/merged",
            utils::get_container_path(&self)?
        );
        let rootfs = rootfs_path_str.as_str();
        info!("mounting container root...");
        mount(
            Some(rootfs),
            rootfs,
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_NOSUID,
            None::<&str>,
        )?;

        info!("changind directory to container root [{}]...", rootfs);
        chdir(rootfs)?;

        info!("container mountpoint prepared successfully...");
        Ok(())
    }
    fn prepare_container_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("preparing container directories...");

        let rootfs_path_str = format!(
            "{}/merged",
            utils::get_container_path(&self)?
        );
        let rootfs = rootfs_path_str.as_str();

        utils::prepare_directory(
            rootfs,
            "put_old",
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IXUSR |
            Mode::S_IRGRP |                 Mode::S_IXGRP |
            Mode::S_IROTH |                 Mode::S_IXOTH
        )?;

        utils::prepare_directory(
            rootfs,
            "dev",
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IXUSR |
            Mode::S_IRGRP |                 Mode::S_IXGRP |
            Mode::S_IROTH |                 Mode::S_IXOTH
        )?;

        utils::prepare_directory(
            rootfs,
            "sys",
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IXUSR |
            Mode::S_IRGRP |                 Mode::S_IXGRP |
            Mode::S_IROTH |                 Mode::S_IXOTH
        )?;

        utils::prepare_directory(
            rootfs,
            "proc",
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IXUSR |
            Mode::S_IRGRP |                 Mode::S_IXGRP |
            Mode::S_IROTH |                 Mode::S_IXOTH
        )?;
        utils::prepare_directory(
            rootfs,
            "old_proc",
            Mode::S_IRUSR | Mode::S_IWUSR | Mode::S_IXUSR |
            Mode::S_IRGRP |                 Mode::S_IXGRP |
            Mode::S_IROTH |                 Mode::S_IXOTH
        )?;

        Ok(())
    }
    fn prepare_container_networking(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("preparing container networking...");

        info!("binding to parent /etc/hosts...");
        if !Path::new("etc/hosts").exists() {
            fs::File::create("etc/hosts")?;
        }
        mount(
            Some("/etc/hosts"),
            "etc/hosts",
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
            None::<&str>,
        )?;

        info!("binding to parent resolv.conf...");
        if !Path::new("etc/resolv.conf").exists() {
            fs::File::create("etc/resolv.conf")?;
        }
        mount(
            Some("/etc/resolv.conf"),
            "etc/resolv.conf",
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_NOSUID | MsFlags::MS_NODEV,
            None::<&str>,
        )?;

        // TODO: Add iproute2 check
        // if true {
        if false {
            // networking::create_network_namespace(&container.id)?;
            networking::create_bridge(&self.id)?;
            networking::create_veth(&self.id)?;
            networking::add_veth_to_bridge(&self.id)?;
        }

        info!("container networking prepared successfuly...");
        Ok(())
    }
    fn mount_container_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("mounting container directories...");

        info!("mounting proc to old_proc...");
        mount(
            Some("/proc"),
            "old_proc",
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_REC,
            None::<&str>,
        )?;

        info!("mounting sys...");
        mount(
            Some("/sys"),
            "sys",
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_REC,
            None::<&str>,
        )?;

        // Slashes?
        info!("mounting dev...");
        mount(
            Some("/dev"),
            "dev",
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_REC,
            None::<&str>,
        )?;

        // info!("populating /dev...");
        // if !Path::new("dev").exists() {
        //     fs::create_dir("dev")?;
        // }
        // let tty_path_str = format!("/dev/tty");
        // let perms =
        //     Mode::S_IRUSR | Mode::S_IWUSR |
        //     Mode::S_IRGRP | Mode::S_IWGRP |
        //     Mode::S_IROTH | Mode::S_IWOTH;
        // if Path::new(tty_path_str.clone().as_str()).exists() {
        //     info!("removing /dev/tty...");
        //     fs::remove_file(tty_path_str.clone())?;
        // }
        // let dev = makedev(5, 0);
        // info!("dev: {}", dev);
        // info!("mknod");
        // mknod(
        //     tty_path_str.clone().as_str(),
        //     SFlag::S_IFCHR,
        //     perms,
        //     dev
        // )?;
        // info!("chown");
        // chown(
        //     tty_path_str.as_str(),
        //     Some(Uid::from_raw(0)),
        //     Some(Gid::from_raw(0))
        // )?;

        // info!("creating ttys");
        // for i in 0..7 {
        //     info!("creating tty{}...", i);

        //     let tty_path_str = format!("/dev/tty{}", i);
        //     let perms =
        //         Mode::S_IRUSR | Mode::S_IWUSR |
        //         Mode::S_IRGRP | Mode::S_IWGRP |
        //         Mode::S_IROTH | Mode::S_IWOTH;
        //     if Path::new(tty_path_str.clone().as_str()).exists() {
        //         info!("removing /dev/tty{}...", i);
        //         fs::remove_file(tty_path_str.clone())?;
        //     }
        //     let dev = makedev(4, i);
        //     info!("dev: {}", dev);
        //     info!("mknod");
        //     mknod(
        //         tty_path_str.clone().as_str(),
        //         SFlag::S_IFCHR,
        //         perms,
        //         dev
        //     )?;
        //     info!("chown");
        //     chown(
        //         tty_path_str.as_str(),
        //         Some(Uid::from_raw(0)),
        //         Some(Gid::from_raw(0))
        //     )?;
        // }

        info!("container directories mounted successfully...");
        Ok(())
    }
    fn prepare_container_ids(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("preparing container ids...");

        info!("uid: {} - euid: {}", Uid::current(), Uid::effective());
        info!("gid: {} - egid: {}", Gid::current(), Gid::effective());

        // let newuid = 3333;
        // let newuid = 1000;
        let newuid = 0;
        // let uid = unistd::getuid();
        let uid = 0;
        let buf = format!("{} {} 1\n", newuid, uid);
        let fd = open("/proc/self/uid_map", OFlag::O_WRONLY, Mode::empty())?;
        info!("writing 'uid_map'");
        write(fd, buf.as_bytes())?;
        close(fd)?;

        let fd = open("/proc/self/setgroups", OFlag::O_WRONLY, Mode::empty())?;
        info!("writing 'deny' to setgroups");
        write(fd, "deny".as_bytes())?;
        close(fd)?;

        // let newgid = 3333;
        // let newgid = 1000;
        let newgid = 0;
        // let gid = unistd::getgid();
        let gid = 0;
        let buf = format!("{} {} 1\n", newgid, gid);
        let fd = open("/proc/self/gid_map", OFlag::O_WRONLY, Mode::empty())?;
        info!("writing 'gid_map' (could fail)");
        write(fd, buf.as_bytes())?;
        close(fd)?;

        // info!("setting ids");
        // let root_uid = unistd::Uid::from_raw(newuid);
        // let root_gid = unistd::Gid::from_raw(newgid);
        // unistd::setresuid(root_uid, root_uid, root_uid)?;
        // unistd::setresgid(root_gid, root_gid, root_gid)?;

        Ok(())
    }
    fn pivot_container_root(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("pivoting container root..");

        info!("pivoting root...");
        pivot_root(".", "put_old")?;

        info!("unmounting pivot auxiliary folder...");
        umount2("/put_old", MntFlags::MNT_DETACH)?;
        if Path::new("/put_old").exists() {
            info!("removing auxiliary folder...");
            std::fs::remove_dir_all("/put_old")?;
        }

        info!("container root pivoted successfully");
        Ok(())
    }
    fn execute_inner_fork(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("executing inner fork...");

        match fork() {
            Ok(ForkResult::Child) => {
                info!("running child process...");

                info!("child pid: {}", getpid());

                self.remount_container_directories()?;

                sethostname("test")?;
                self.do_exec("/bin/sh")?;
                // self.do_exec("/sbin/init")?;

                // Should not reach
                info!("exiting child process...");
                std::process::exit(0);
            }
            Ok(ForkResult::Parent { child, .. }) => {
                info!("running parent process...");
                info!("inner fork child pid: {}", child);

                info!("waiting for child...");
                waitpid(child, None)?;
            }

            Err(_) => {}
        };

        info!("inner fork executed successfully");
        Ok(())
    }
    fn remount_container_directories(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("remounting container directories...");

        info!("remounting proc...");
        mount(
            Some("/proc"),
            "/proc",
            Some("proc"),
            MsFlags::MS_NOSUID,
            None::<&str>,
        )?;

        info!("unmounting old proc folder...");
        umount2("/old_proc", MntFlags::MNT_DETACH)?;
        info!("removing old proc folder...");
        if Path::new("/old_proc").exists() {
            info!("removing old proc folder...");
            std::fs::remove_dir_all("/old_proc")?;
        }

        info!("mounting remounting container root...");
        mount(
            Some("/"),
            "/",
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_RDONLY | MsFlags::MS_NOSUID | MsFlags::MS_REMOUNT,
            None::<&str>,
        )?;

        info!("container directories remounted successfully...");
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
        info!("arguments: \n{:?}\n{:?}\n{:?}",
            args, envs, p);
        execve(&p, &a, &e)?;

        Ok(())
    }
    fn unmount_container_filesystem(&self) -> Result<(), Box<dyn std::error::Error>> {
        let merged = format!("{}/merged", &self.path);

        info!("unmounting '{}'...", merged);
        // umount2(merged.as_str(), MntFlags::MNT_DETACH)?;
        // umount2(merged.as_str(), MntFlags::MNT_FORCE)?;
        umount(merged.as_str())?;

        Ok(())
    }
    fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("cleaning up container...");

        // chdir("/")?;
        self.unmount_container_filesystem()?;

        info!("cleanup successful");
        Ok(())
    }
    fn clean_run(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.mount_container_filesystem()?;

        self.prepare_container_mountpoint()?;

        self.prepare_container_directories()?;

        self.prepare_container_networking()?;

        self.mount_container_directories()?;

        self.prepare_container_ids()?;

        self.pivot_container_root()?;

        self.execute_inner_fork()?;

        // TODO: Move code where it belongs(???)
        // if true {
        if false {
            networking::delete_container_from_network(&self.id)?;
            networking::remove_veth_from_bridge(&self.id)?;
            networking::delete_veth(&self.id)?;
            networking::delete_bridge(&self.id)?;
            networking::delete_network_namespace(&self.id)?;
        }

        Ok(())
    }
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("running container...");

        info!("executing outer fork...");
        let result = match fork() {
            Ok(ForkResult::Child) => {
                self.clean_run()?;
                std::process::exit(0)
            }
            Ok(ForkResult::Parent { child, .. }) => {
                info!("outer fork child pid: {}", child);
                waitpid(child, None)?;
                Ok(())
            }
            Err(e) => Err(From::from(e))
        };
        info!("outer fork executed successfully");

        self.cleanup()?;

        result
        // match result {
        //     Err(e) => {
        //         info!("error encountered while running container: {}", e);
        //         Ok(())
        //     }
        //     Ok(()) => {
        //         Ok(())
        //     }
        // }
    }


    pub fn delete(&self,) -> Result<(), Box<dyn std::error::Error>> {
        info!("deleting container '{}'...", &self.id);

        let container_path = Path::new(&self.path);
        if !container_path.exists() {
            info!("container not found. skipping deletion...");
            return Ok(())
        }

        fs::remove_dir_all(container_path)?;

        info!("deletion successfull");
        Ok(())
    }
}
