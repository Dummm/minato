use log::info;
use std::process::Command;
use std::io::{self, prelude::*};
use nix::unistd;
use std::os::unix;
use std::fs;
use std::path::Path;

#[allow(dead_code)]
/// Create a network namespace for tha container
pub fn create_network_namespace(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating network namespace...");
    let namespace = format!("{}-ns", container_id);

    // ip netns add {ns}
    let output = Command::new("ip").arg("netns").arg("add").arg(namespace)
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("created network namespace.");
    Ok(())
}
/// Delete the container's network namespace
pub fn delete_network_namespace(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("deleting network namespace...");
    let namespace = format!("{}-ns", container_id);

    info!("ip netns del {}", namespace);
    let output = Command::new("ip").arg("netns").arg("del").arg(namespace)
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("deleted network namespace.");
    Ok(())
}

// TODO: Replace hardcode
// TODO: Combine command arguments
/// Create network bridge
pub fn create_bridge(container_id: &str, host_ip: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating bridge...");

    // // let host_ip = "192.168.1.10/24";
    // let host_ip = "10.1.0.1/16";
    // let bridge_name = format!("{}-br0", container_id);
    let bridge_name = "br0";

    info!("ip link add {} type bridge", bridge_name);
    let output = Command::new("ip").arg("link").arg("add").arg(bridge_name.clone()).arg("type").arg("bridge")
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("ip addr add {} dev {}", host_ip, bridge_name);
    let output = Command::new("ip").arg("addr").arg("add").arg(host_ip).arg("dev").arg(bridge_name.clone())
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("ip link set dev {} up", bridge_name);
    let output = Command::new("ip").arg("link").arg("set").arg("dev").arg(bridge_name).arg("up")
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("created bridge.");
    Ok(())
}
/// Delete network bridge
pub fn delete_bridge(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("deleting bridge...");

    // let bridge_name = format!("{}-br0", container_id);
    let bridge_name = "br0";

    info!("ip link del name {}", bridge_name);
    let output = Command::new("ip").arg("link").arg("del").arg("name").arg(bridge_name)
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("deleted bridge.");
    Ok(())
}

/// Create virtual ethernet device between host and container
pub fn create_veth(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating veth...");

    let veth_host = format!("{}-veth0", container_id);
    // let host_ip = "192.168.1.10/24";
    let veth_guest = format!("{}-veth1", container_id);

    info!("ip link add {} type veth peer name {}", veth_host, veth_guest);
    let output = Command::new("ip").arg("link").arg("add")
        .arg(veth_host.clone()).arg("type").arg("veth").arg("peer").arg("name").arg(veth_guest)
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    // info!("ip addr add {} dev {}", host_ip, veth_host);
    // let output = Command::new("ip").arg("addr").arg("add").arg(host_ip).arg("dev").arg(veth_host.clone())
    //     .output()
    //     .unwrap();
    // info!("output: {}", output.status);
    // io::stdout().write_all(&output.stdout).unwrap();
    // io::stderr().write_all(&output.stderr).unwrap();

    info!("ip link set {} up", veth_host);
    let output = Command::new("ip").arg("link").arg("set").arg(veth_host).arg("up")
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("created veth.");
    Ok(())
}
/// Delete virtual ethernet device between host and container
pub fn delete_veth(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("deleting veth...");

    let namespace = format!("{}-ns", container_id);
    let veth_host = format!("{}-veth0", container_id);

    info!("ip netns exec {} ip link del {}", namespace, veth_host);
    let output = Command::new("ip").arg("netns").arg("exec").arg(namespace.clone())
        .arg("ip").arg("link").arg("del").arg(veth_host.clone())
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("ip link del {}", veth_host);
    let output = Command::new("ip").arg("link").arg("del").arg(veth_host)
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("deleted veth.");
    Ok(())
}

/// Add the virtual ethernet to the bridge
pub fn add_veth_to_bridge(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("adding veth to bridge...");

    let veth_host = format!("{}-veth0", container_id);
    // let bridge_name = format!("{}-br0", container_id);
    let bridge_name = "br0";

    info!("ip link set dev {} master {}", veth_host, bridge_name);
    let output = Command::new("ip").arg("link").arg("set").arg("dev").arg(veth_host).arg("master").arg(bridge_name)
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("added veth to bridge.");
    Ok(())
}
/// Remove the virtual ethernet from the bridge
pub fn remove_veth_from_bridge(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("removing veth from bridge...");

    let veth_host = format!("{}-veth0", container_id);

    info!("ip link set dev {} nomaster", veth_host);
    let output = Command::new("ip").arg("link").arg("set").arg("dev").arg(veth_host).arg("nomaster")
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("removed veth from bridge.");
    Ok(())
}

#[allow(dead_code)]
/// Add the container to the network namespace
pub fn add_container_to_network(container_id: &str, child: unistd::Pid, container_ip: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("adding container to network...");

    let namespace = format!("{}-ns", container_id);
    // let container_ip = "192.168.1.11/24";
    // let container_ip2 = "192.168.1.11";
    // let container_ip2 = "10.1.0.2/16";
    let container_ip2 = &container_ip
        .split('/')
        .map(|str| String::from(str))
        .collect::<Vec<String>>()[0];
    let veth_guest = format!("{}-veth1", container_id);

    info!("ln /proc/{}/ns/net /var/run/netns/{}", child, namespace);
    let netns_path_str = "/var/run/netns/";
    if !Path::new(netns_path_str).exists() {
        fs::create_dir_all(netns_path_str)?;
    }

    let ns_path = format!("/var/run/netns/{}", namespace);
    info!("{}", ns_path);
    if let Err(e) = fs::remove_file(ns_path) {
        info!("{}", e);
    }
    unix::fs::symlink(
        format!("/proc/{}/ns/net", child),
        format!("/var/run/netns/{}", namespace)
    )?;

    info!("ip link set {} netns {}", veth_guest, namespace);
    let output = Command::new("ip").arg("link").arg("set").arg(veth_guest.clone()).arg("netns").arg(namespace.clone())
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

	info!("ip netns exec {} ip link set lo up", namespace);
    let output = Command::new("ip").arg("netns").arg("exec").arg(namespace.clone())
        .arg("ip").arg("link").arg("set").arg("lo").arg("up")
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("ip netns exec {} ip link set {} up", namespace, veth_guest);
    let output = Command::new("ip").arg("netns").arg("exec").arg(namespace.clone())
        .arg("ip").arg("link").arg("set").arg(veth_guest.clone()).arg("up")
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("ip netns exec {} ip addr add {} dev {}", namespace, container_ip, veth_guest);
    let output = Command::new("ip").arg("netns").arg("exec").arg(namespace.clone())
        .arg("ip").arg("addr").arg("add").arg(container_ip).arg("dev").arg(veth_guest)
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("ip netns exec {} ip route add default via {}", namespace, container_ip2);
    let output = Command::new("ip").arg("netns").arg("exec").arg(namespace)
        .arg("ip").arg("route").arg("add").arg("default").arg("via").arg(container_ip2)
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("added container to network.");
    Ok(())
}
/// Delete the container from the network namespace
pub fn delete_container_from_network(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("deleting container from network...");

    let namespace = format!("{}-ns", container_id);

    info!("ip netns exec {} ip route del default", namespace);
    let output = Command::new("ip").arg("netns").arg("exec").arg(namespace)
        .arg("ip").arg("route").arg("del").arg("default")
        .output()
        .unwrap();
    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    info!("deleted container from network.");
    Ok(())
}