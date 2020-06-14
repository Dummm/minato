use std::iter;
use std::fs;
// use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::os::unix;
use std::ffi::CString;
use std::env;
use nix::mount::{mount, MntFlags, MsFlags, umount, umount2};
use nix::sched::{CloneFlags, unshare};
use nix::sys::stat::Mode;
use std::process::exit;
// use nix::sys::stat::{Mode, makedev, mknod, SFlag};
use nix::sys::wait::waitpid;
use nix::unistd::*;
use nix::fcntl::{open, OFlag};
#[allow(unused_imports)]
use rand::{distributions::Alphanumeric, thread_rng, Rng};

// use dirs;
use log::{info, error, debug};

use crate::image::Image;
use crate::utils;
use crate::networking;
use crate::spec::Spec;
use crate::spec::Namespace;
use crate::spec::NamespaceType;



pub struct Container {
    pub id: String,
    pub image: Option<Image>,
    pub path: String,
    pub spec: Spec,
    // pub state: State,
}
impl Container {
    /// Create a new container object
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
        let spec = Spec::new().unwrap();

        Container {
            id,
            image,
            path,
            spec,
            // state: State::Stopped,
        }
    }

    /// Create a default config.json by saving the one in the project root. (It's loaded by default on object creation)
    fn generate_config_json(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("creating config json...");

        // let spec = Spec::new()?;
        // spec.save(&self.path)?;
        let spec_path = format!("{}/config.json", &self.path);
        self.spec.save(spec_path.as_str())?;

        info!("created config json.");
        Ok(())
    }
    /// Create a directory to download and store the container
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

        info!("created container directory structure.");
        Ok(())
    }
    /// Create and store a container
    pub fn create(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("creating container");
        debug!("container fields: {}, {}'...", &self.id, &self.path);

        if Path::new(&self.path).exists() {
            info!("container exists. skipping creation...");
            return Ok(())
        }

        self.create_directory_structure()?;
        self.generate_config_json()?;

        info!("created container.");
        Ok(())
    }

    // TODO: Find a better way to find image
    /// Load a stored container
    pub fn load(container_name: &str) -> Result<Option<Container>, Box<dyn std::error::Error>> {
        let mut container = Container::new(Some(container_name), None);

        let container_path = Path::new(&container.path);
        if !container_path.exists() {
            return Ok(None)
        }

        let container_lower_path = container_path.join("lower");
        let container_image_path = container_lower_path.read_link().unwrap();

        // let home = match dirs::home_dir() {
        //     Some(path) => path,
        //     None       => return Err("error getting home directory".into())
        // };
        // let images_path = format!(
        //     "{}/.minato/images/",
        //     home.display()
        // );
        debug!("{}", container_image_path.display());
        let images_path = format!("/var/lib/minato/images/");

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

        let spec_path = format!("{}/config.json", container.path);
        container.spec = Spec::load(spec_path.as_str())?;

        Ok(Some(container))
    }

    /// Mount the container layers in a single directory using overlayfs
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
        debug!("mount arguments: \n{}\n{}\n{}\n{}",
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

        info!("mounted container filesystem.");
        Ok(())
    }
    /// Prepare the container root
    ///
    /// Executed steps:
    ///   - unsharing the namespaces
    ///   - making the parent root private
    ///   - mounting the container root
    ///   - changing directory to container root
    ///   - creating a 'tini' file and mount-binding it to the one in the .minato directoryg
    fn prepare_container_mountpoint(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("preparing container mountpoint...");

        // TODO: Process network ns too
        let mut clone_flags = CloneFlags::empty();
        let namespaces: &Vec<Namespace> = &self.spec.linux.as_ref().unwrap().namespaces;
        for ns in namespaces {
            match ns.typ {
                NamespaceType::pid     => clone_flags |= CloneFlags::CLONE_NEWPID,
                NamespaceType::mount   => clone_flags |= CloneFlags::CLONE_NEWNS,
                NamespaceType::uts     => clone_flags |= CloneFlags::CLONE_NEWUTS,
                NamespaceType::ipc     => clone_flags |= CloneFlags::CLONE_NEWIPC,
                NamespaceType::user    => clone_flags |= CloneFlags::CLONE_NEWUSER,
                NamespaceType::cgroup  => clone_flags |= CloneFlags::CLONE_NEWCGROUP,
                NamespaceType::network => clone_flags |= CloneFlags::CLONE_NEWNET,
                // _ => {}
            }
        }

        info!("unsharing parent namespaces");
        unshare(clone_flags)?;

        info!("making parent root private");
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
        info!("mounting container root");
        mount(
            Some(rootfs),
            rootfs,
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_NOSUID,
            None::<&str>,
        )?;

        info!("changing directory to container root [{}]...", rootfs);
        chdir(rootfs)?;

        // TODO: Move?
        // let home = match dirs::home_dir() {
        //     Some(path) => path,
        //     None       => return Err("error getting home directory".into())
        // };
        // let tini_path = format!(
        //     "{}/.minato/tini",
        //     home.display()
        // );
        let tini_path = format!("/var/lib/minato/tini");
        info!("binding init executable to container...");
        let tini_bin = "sbin/tini";
        if !Path::new(&tini_bin).exists() {
            fs::File::create(&tini_bin)?;
            // fs::remove_file("tini")?;
        }
        // let f = fs::File::create("tini")?;
        // let metadata = f.metadata()?;
        // let mut permissions = metadata.permissions();
        // permissions.set_mode(0o777);
        // info!("{:o}", permissions.mode());

        // }
        mount(
            Some(tini_path.as_str()),
            tini_bin,
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_NOSUID | MsFlags::MS_NODEV, // | MsFlags::MS_RDONLY,
            None::<&str>,
        )?;
        // let f = fs::File::open("tini")?;
        // let metadata = f.metadata()?;
        // let mut permissions = metadata.permissions();
        // permissions.set_mode(0o777);
        // info!("{:o}", permissions.mode());
        // chown(
        //     tini_path.as_str(),

        // )?;

        info!("prepared container mountpoint.");
        Ok(())
    }
    /// Create container root directories for future actions
    ///
    /// Directories: put_old, dev, sys, proc, old_proc
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

        info!("prepared container directories.");
        Ok(())
    }
    /// Prepare container for networking, communication with host network
    ///
    /// Executed steps:
    ///   - binding /etc/hosts and /etc/resolv.conf to the same files in the parent
    ///   - TO BE REENABLED: executing iproute2 commands to establish connection
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

        info!("prepared container networking.");
        Ok(())
    }
    /// Mount all the neccessary cgroup directories in /sys/fs/cgroup
    ///
    /// The directories are populated automatically by the kernel
    fn mount_container_cgroup_hierarchy(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("mounting container cgroup hierarchy...");

        let directories = vec![
            // "unified", // cgroup2
            // "systemd",
            "freezer",
            "hugetlb",
            "memory",
            "blkio",
            "cpuset",
            "cpu,cpuacct",
            "devices",
            "pids",
            "net_cls,net_prio",
            "perf_event",
            "rdma"
        ];

        for dir in directories {
            let dir_path = format!("sys/fs/cgroup/{}", dir);
            if Path::new(&dir_path).exists() {
                fs::remove_dir_all(&dir_path)?;
            }
            fs::create_dir_all(&dir_path)?;

            info!("mounting cgroup {}...", dir);
            let cgroup_version = "cgroup";
            mount(
                Some(cgroup_version),
                dir_path.as_str(),
                Some(cgroup_version),
                MsFlags::MS_NOSUID | MsFlags::MS_NODEV | MsFlags::MS_NOEXEC,
                Some(dir),
            )?;
        }

        info!("mounted container cgroup hierarchy.");
        Ok(())
    }
    #[allow(dead_code)]
    /// NOT WORKING: Probably because of loss of privilages
    ///
    /// Change the cgroup values according to the container's config.json file
    fn configure_cgroups(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("configuring cgroups...");

        let cgroup_path = format!("sys/fs/cgroup");
        let resources = match &self.spec.linux.as_ref().unwrap().resources {
            None => {
                info!("no resources found. skipping...");
                return Ok(())
            },
            Some(res) => res
        };

        info!("configuring devices cgroup...");
        let devices = &resources.devices;
        // let devices = match resources.devices {
        //     None => {
        //         info!("no devices found. skipping...");
        //         return Ok(())
        //     },
        //     Some(dev) => dev
        // };
        for device in devices {
            debug!("{:?}", device);
            let file = if device.allow {
                "devices.allow"
            } else {
                "devices.deny"
            };
            // TODO: Necessary?
            let typ = device.typ.as_str();
            let major = match device.major {
                Some(x) => x.to_string(),
                None    => "*".to_string()
            };
            let minor = match device.minor {
                Some(x) => x.to_string(),
                None    => "*".to_string()
            };
            let acc = device.access.as_str();
            let data = format!{"{} {}:{} {}", typ, &major, &minor, acc};

            let devices_path = format!("{}/devices/{}", cgroup_path, file);
            info!{"writing {} to {}", data, devices_path};
            if let Err(e) = fs::write(devices_path, data.as_bytes()) {
                error!("{}", e);
            }
        }

        info!("configured cgroups.");
        Ok(())
    }
    /// Execute various mount operations
    ///
    /// Mounts:
    ///   - proc fs to old_proc
    ///   - parent /dev to /dev
    ///
    /// Other options:
    ///   - /sys to /sys
    ///   - not mounting dev to the parent and populating /dev manually
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

        self.mount_container_cgroup_hierarchy()?;
        debug!("TODO: cgroups configuring");
        // self.configure_cgroups()?;

        // info!("mounting sys...");
        // mount(
        //     Some("/sys"),
        //     "sys",
        //     None::<&str>,
        //     MsFlags::MS_BIND | MsFlags::MS_REC,
        //     None::<&str>,
        // )?;

        // Slashes?
        info!("mounting dev to dev...");
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

        // BUG: Init doesn't work
        // if self.spec.process.args[0] == "init" {
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

        info!("mounted container directories.");
        Ok(())
    }
    /// Write the container maps
    ///
    /// 'uid_map' and 'gid_map' are a 1-1 mapping, root(0) to the configured id
    ///
    /// Setting 'setgroups' to 'deny' is required to get the user namespace to work
    fn prepare_container_id_maps(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("preparing container id maps...");

        debug!("uid: {} - euid: {}", Uid::current(), Uid::effective());
        debug!("gid: {} - egid: {}", Gid::current(), Gid::effective());

        // let uid = unistd::getuid();
        let uid = 0;
        let newuid = self.spec.process.user.uid;
        let buf = format!("{} {} 1\n", newuid, uid);
        let fd = open("/proc/self/uid_map", OFlag::O_WRONLY, Mode::empty())?;
        info!("writing 'uid_map'");
        write(fd, buf.as_bytes())?;
        close(fd)?;

        let fd = open("/proc/self/setgroups", OFlag::O_WRONLY, Mode::empty())?;
        info!("writing 'deny' to setgroups");
        write(fd, "deny".as_bytes())?;
        close(fd)?;

        // let gid = unistd::getgid();
        let gid = 0;
        let newgid = self.spec.process.user.gid;
        let buf = format!("{} {} 1\n", newgid, gid);
        let fd = open("/proc/self/gid_map", OFlag::O_WRONLY, Mode::empty())?;
        info!("writing 'gid_map'");
        write(fd, buf.as_bytes())?;
        close(fd)?;

        // info!("setting groups...");
        // let gids: Vec<Gid> = self.spec.process.user.additional_gids.iter()
        //     .map(|gid| Gid::from_raw(*gid as u32))
        //     .collect();
        // setgroups(gids.as_slice())?;

        info!("prepared container id maps.");
        Ok(())
    }
    #[allow(dead_code)]
    /// NOT WORKING: Not sure if it's supposed to work or not. Probably because of loss of privileges also
    ///
    /// Set the configured uid and gids
    fn prepare_container_ids(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Err(e) = prctl::set_keep_capabilities(true) {
            info!("failed to set keep capabilities to true: {}", e);
            return Ok(())
        };

        let newuid = self.spec.process.user.uid;
        let newgid = self.spec.process.user.gid;

        info!("setting ids...");
        let root_uid = Uid::from_raw(newuid as u32);
        let root_gid = Gid::from_raw(newgid as u32);
        setresuid(root_uid, root_uid, root_uid)?;
        setresgid(root_gid, root_gid, root_gid)?;

        info!("uid: {} - euid: {}", Uid::current(), Uid::effective());
        info!("gid: {} - egid: {}", Gid::current(), Gid::effective());

        if let Err(e) = prctl::set_keep_capabilities(false) {
            info!("failed to set keep capabilities to false: {}", e);
            return Ok(())
        };
        Ok(())
    }
    /// Pivot root to the container's root and unmount the auxilliary 'put_old' folder after
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

        info!("pivoted container root.");
        Ok(())
    }
    /// Execute the inner fork, before executing the initial command
    ///
    /// Probably to enter all the namespaces after unsharing them
    fn execute_inner_fork(&self, daemon: bool) -> Result<(), Box<dyn std::error::Error>> {
        info!("executing inner fork...");

        match fork() {
            Ok(ForkResult::Child) => {
                info!("running child process...");

                self.remount_container_directories()?;

                sethostname(self.spec.hostname.as_str())?;

                // if let Err(e) = self.prepare_container_ids() {
                //     info!("failed: {}", e);
                // }

                self.do_exec()?;

                // Should not reach
                error!("exited child process. (should not reach!)");
                std::process::exit(0);
            }
            Ok(ForkResult::Parent { child, .. }) => {
                info!("running parent process...");

                info!("inner fork child pid: {}", child);

                if !daemon {
                    info!("waiting for child...");
                    waitpid(child, None)?;
                    exit(0);
                }
            }
            Err(e) => error!("inner fork error: {}", e)
        };

        info!("executed inner fork.");
        Ok(())
    }
    /// Remount directories, as a child process
    ///
    /// Remounts:
    ///   - proc fs to 'proc', from 'old_proc'; then removes 'old_proc'
    ///   - the root directory
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

        // BUG: Unsure if it's needed. Probably for userns
        info!("remounting container root...");
        mount(
            Some("/"),
            "/",
            None::<&str>,
            MsFlags::MS_BIND | MsFlags::MS_NOSUID | MsFlags::MS_REMOUNT,
            None::<&str>,
        )?;

        info!("remounted container directories.");
        Ok(())
    }
    /// Execute the initial command, usually '/bin/sh'.
    ///
    /// Actually also runs an init process (tini) before, for more process management and initial command in daemon
    ///
    /// Daemon option not working yet
    fn do_exec(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("preparing command execution...");

        let mut tini = vec![String::from("tini")];

        // let process = &self.spec.process;
        // let path = &process.args[0];
        // let args = &process.args;
        // let envs = &process.env;

        // let process = &self.spec.process;
        // let path = if self.spec.process.args[0] == "init" {
        //     &tini[0]
        // } else {
        //     &process.args[0]
        // };
        // let args = if self.spec.process.args[0] == "init" {
        //     &tini
        // } else {
        //     &process.args
        // };
        // let envs = &process.env;

        let process = &self.spec.process;
        tini.append(&mut process.args.clone());
        let path = "tini";
        let args = tini;
        let envs = &process.env;

        let p: CString = CString::new(path).unwrap();
        let a: Vec<CString> = args.iter()
            .map(|s| CString::new(s.to_string()).unwrap_or_default())
            .collect();
        let e: Vec<CString> = envs.iter()
            .map(|s| CString::new(s.to_string()).unwrap_or_default())
            .collect();

        info!("setting environment variables...");
        for env in envs {
            let e: Vec<&str> = env.split("=").collect();
            let variable = e[0];
            let value = e[1];

            env::remove_var(variable);
            env::set_var(variable, value);
        }

        info!("executing command...");
        info!("arguments: \n{:?}\n{:?}\n{:?}",
            p, a, e);
        execvpe(&p, &a, &e)?;

        Ok(())
    }
    /// Unmount the overlay filesystem
    fn unmount_container_filesystem(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("unmounting container filesystem...");
        let merged = format!("{}/merged", &self.path);

        info!("unmounting '{}'...", merged);
        umount(merged.as_str())?;

        info!("unmounted container filesystem.");
        Ok(())
    }
    /// Cleanup after running the container
    ///
    /// Executed steps:
    ///   - unmount the overlay filesystem
    ///   - remove the pid file from the container directory
    fn cleanup(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("cleaning up container...");

        // chdir("/")?;
        self.unmount_container_filesystem()?;

        // TODO: Move code where it belongs(???)
        if true {
        // if false {
            networking::delete_container_from_network(&self.id)?;
            networking::remove_veth_from_bridge(&self.id)?;
            networking::delete_veth(&self.id)?;
            networking::delete_bridge(&self.id)?;
            networking::delete_network_namespace(&self.id)?;
        }

        let pid_path = format!("{}/pid", self.path);
        if Path::new(&pid_path).exists() {
            info!("removing pid file...");
            fs::remove_file(&pid_path)?;
        }

        info!("cleaned up (after) container.");
        Ok(())
    }
    /// Separate run function necessary to execute a fork and cleanup
    ///
    /// Executed steps:
    ///   - mount the container filesystem
    ///   - prepare the container root mountpoint
    ///   - prepare the container root directories
    ///   - prepare the container root directories
    ///   - prepare the container for networking
    ///   - mount container directories
    ///   - write the container id maps
    ///   - pivot root
    ///   - execute inner fork
    ///     - remount proc and root
    ///     - set hostname
    ///     - execute initial command
    fn clean_run(&self, daemon: bool) -> Result<(), Box<dyn std::error::Error>> {

        debug!("uid: {} - euid: {}", Uid::current(), Uid::effective());
        debug!("gid: {} - egid: {}", Gid::current(), Gid::effective());

        self.mount_container_filesystem()?;

        self.prepare_container_mountpoint()?;

        self.prepare_container_directories()?;

        self.prepare_container_networking()?;

        self.mount_container_directories()?;

        self.prepare_container_id_maps()?;

        self.pivot_container_root()?;

        self.execute_inner_fork(daemon)?;


        Ok(())
    }
    /// Run a stored container
    ///
    /// Executes a fork before all the steps so it works with a daemon
    ///
    /// The parent creates a pid file that is used to check the container's state
    pub fn run(&self, daemon: bool) -> Result<(), Box<dyn std::error::Error>> {
        info!("running container...");

        info!("executing outer fork...");
        let result = match fork() {
            Ok(ForkResult::Child) => {
                self.clean_run(daemon)?;

                return Ok(());
            }
            Ok(ForkResult::Parent { child, .. }) => {
                info!("outer fork child pid: {}", child);

                // TODO: Add iproute2 check
                if true {
                // if false {
                    // networking::create_network_namespace(&container.id)?;
                    networking::create_bridge(&self.id)?;
                    networking::create_veth(&self.id)?;
                    networking::add_veth_to_bridge(&self.id)?;
                    networking::add_container_to_network(&self.id, child)?;
                }

                info!("writing pid file...");
                let pid_path = format!("{}/pid", self.path);
                if Path::new(&pid_path).exists() {
                    info!("removing pid file...");
                    fs::remove_file(&pid_path)?;
                }
                let pid_str = format!("{}\n", child.as_raw().to_string());
                fs::File::create(&pid_path)?;
                fs::write(pid_path, pid_str)?;

                std::thread::sleep(std::time::Duration::from_millis(10));
                // Path::new("/proc/self/ns")
                //     .read_dir().unwrap()
                //     .for_each(|dir| {
                //         let ns_path = format!("{}",
                //             dir.unwrap()
                //             .path()
                //             .display());
                //         let link = format!("{}", fs::read_link(&ns_path).unwrap().display());
                //         let ns_dir_path = format!("{}/{}",
                //             &self.path,
                //             Path::new(&ns_path)
                //                 .strip_prefix(Path::new("/proc/self/ns/")).unwrap()
                //                 .display()
                //             );

                //         info!("{} -> {}", ns_dir_path, link);
                //         // if let Err(e) = fs::copy(ns_path, ns_dir_path) {
                //         //     info!("{}", e);
                //         // };
                //         if let Err(e) = fs::soft_link(ns_dir_path, link) {
                //             info!("{}", e);
                //         };
                //     });

                if !daemon {
                    waitpid(child, None)?;
                    // exit(0);
                }

                Ok(())
            }
            Err(e) => Err(From::from(e))
        };
        info!("executed outer fork.");

        self.cleanup()?;

        info!("ran container.");
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

    /// Delete a stored container
    pub fn delete(&self,) -> Result<(), Box<dyn std::error::Error>> {
        info!("deleting container '{}'...", &self.id);

        let container_path = Path::new(&self.path);
        if !container_path.exists() {
            info!("container not found. skipping deletion...");
            return Ok(())
        }

        fs::remove_dir_all(container_path)?;

        info!("deleted container");
        Ok(())
    }
}
