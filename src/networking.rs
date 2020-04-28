use log::info;
use std::process::Command;
use std::io::{self, prelude::*};

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

    Ok(())
}

pub fn delete_network_namespace(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("deleting network namespace...");
    let namespace = format!("{}-ns", container_id);

    // ip netns del {ns}
    let output = Command::new("ip").arg("netns").arg("del").arg(namespace)
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    Ok(())
}

// TODO: Replace hardcode
// TODO: Combine command arguments
pub fn create_bridge(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating bridge...");

    // let namespace = format!("{}-ns", container_id);
    let bridge_ip = "172.0.0.1/16"; // {ip}/16
    let bridge_name = format!("{}-br0", container_id);


	// ip link add name {bname} type bridge
    let output = Command::new("ip").arg("link").arg("add").arg(bridge_name.clone()).arg("type").arg("bridge")
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

	// ip addr add {bridge_ip}/16 dev {bname}
    let output = Command::new("ip").arg("addr").arg("add").arg(bridge_ip).arg("dev").arg(bridge_name.clone())
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    // ip link set dev {bname} up
    let output = Command::new("ip").arg("link").arg("set").arg("dev").arg(bridge_name).arg("up")
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    Ok(())
}

pub fn delete_bridge(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("deleting bridge...");

    let bridge_name = format!("{}-br0", container_id);

	// ip link del name {bname}
    let output = Command::new("ip").arg("link").arg("del").arg("name").arg(bridge_name).arg("up")
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    Ok(())
}

pub fn create_veth(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("creating veth...");

    let veth_host = format!("{}-veth0", container_id);
    let veth_guest = format!("{}-veth1", container_id);

    // ip link add {ve_host} type veth peer name {ve_guest}
    let output = Command::new("ip").arg("link").arg("add")
        .arg(veth_host.clone()).arg("type").arg("veth").arg("peer").arg("name").arg(veth_guest)
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

	// ip link set {ve_host} up
    let output = Command::new("ip").arg("link").arg("set").arg(veth_host).arg("up")
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    Ok(())
}

pub fn delete_veth(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("deleting veth...");

    let veth_host = format!("{}-veth0", container_id);

    // ip link del {ve_host}
    let output = Command::new("ip").arg("link").arg("del").arg(veth_host)
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    Ok(())
}

pub fn add_veth_to_bridge(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("adding veth to bridge...");

    let veth_host = format!("{}-veth0", container_id);
    let bridge_name = format!("{}-br0", container_id);

    // ip link set dev {ve_host} master {bname}
    let output = Command::new("ip").arg("link").arg("set").arg("dev").arg(veth_host).arg("master").arg(bridge_name)
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    Ok(())
}

pub fn remove_veth_from_bridge(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("removing veth from bridge...");

    let veth_host = format!("{}-veth0", container_id);

    // ip link set dev {ve_host} nomaster
    let output = Command::new("ip").arg("link").arg("set").arg("dev").arg(veth_host).arg("nomaster")
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    Ok(())
}

pub fn add_container_to_network(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("adding container to network...");

    let namespace = format!("{}-ns", container_id);
    let container_ip = "172.0.0.2/16"; // {ip}/16
    let bridge_ip = "172.0.0.1/16"; // {ip}/16
    // let bridge_name = format!("{}-br0", container_id);
    // let veth_host = format!("{}-veth0", container_id);
    let veth_guest = format!("{}-veth1", container_id);


	// (ip netns exec {ns})? ip link set {ve_guest} netns {pid? ns?}
    let output = Command::new("ip").arg("netns").arg("exec").arg(namespace.clone())
    .arg("ip").arg("link").arg("set").arg(veth_guest.clone()).arg("netns").arg(namespace.clone())
    // let output = Command::new("ip").arg("netns").arg("exec").arg(namespace.clone())
        // .arg("ip").arg("netns").arg("list")
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

	// ip netns exec {ns} ip link set lo up
    let output = Command::new("ip").arg("netns").arg("exec").arg(namespace.clone())
        .arg("ip").arg("link").arg("set").arg("lo").arg("up")
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    // ip link set {ve_guest} netns {ns} up
    let output = Command::new("ip").arg("link").arg("set").arg(veth_guest.clone()).arg("up")
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    // ip netns exec {ns} ip addr add {cont_ip}/16 dev {ve_guest}
    let output = Command::new("ip").arg("netns").arg("exec").arg(namespace.clone())
        .arg("ip").arg("addr").arg("add").arg(container_ip).arg("dev").arg(veth_guest)
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    // ip netns exec {ns} ip route add default via {bridge_ip}
    let output = Command::new("ip").arg("netns").arg("exec").arg(namespace)
        .arg("ip").arg("route").arg("add").arg("default").arg("via").arg(bridge_ip)
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    Ok(())
}

pub fn delete_container_from_network(container_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("deleting container from network...");

    let namespace = format!("{}-ns", container_id);

    // ip netns exec {ns} ip route del default
    let output = Command::new("ip").arg("netns").arg("exec").arg(namespace)
        .arg("ip").arg("route").arg("del").arg("default")
        .output()
        .unwrap();

    info!("output: {}", output.status);
    io::stdout().write_all(&output.stdout).unwrap();
    io::stderr().write_all(&output.stderr).unwrap();

    Ok(())
}