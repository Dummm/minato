use std::fs::{self, File};
use std::path::Path;
use std::io;
// use std::io::{self, Error, ErrorKind};
// use std::error;

use log::info;
use tar::Archive;
use flate2::read::GzDecoder;
use reqwest;
use serde_json::{self, Value};
use dirs;


use crate::image::Image;

fn get_authentication_token(auth_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    info!("sending authentication token request to: {}...", auth_url);

    let response = reqwest::blocking::get(auth_url)?;
    let response_text = response.text()?;
    let body: Value = serde_json::from_str(response_text.as_str())?;
    // let body: Value = serde_json::from_str(response_text.as_str()) {
    //     Ok(body) => body,
    //     Err(_)   => return Err("json parsing failed".to_string())
    // };
    info!("parsed json successfully");

    let token = match &body["token"] {
        Value::String(t) => t,
        _ => return Err("token retrieval failed".into()),
    };
    info!("retrieved token successfully");

    Ok(token.clone())
}

fn get_filesystem_layers(token: &str, manifests_url: &str) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    info!("sending manifests request to: {}...", manifests_url);

    let response = reqwest::blocking::Client::new()
        .get(manifests_url)
        .bearer_auth(token)
        .send()?;
    let response_text = response.text()?;
    let body: Value = serde_json::from_str(response_text.as_str())?;

    let fs_layers = match &body["fsLayers"] {
        Value::Array(fs_layers) => fs_layers,
        _ => return Err("filesystem layers retrieval failed".into()),
    };

    Ok(fs_layers.clone())
}

fn get_image_path(image: &Image) -> Result<String, Box<dyn std::error::Error>> {
    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };

    Ok(format!(
        "{}/.minato/images/{}:{}",
        home.display(), image.name, image.reference
    ))
}

fn download_layer(image: &mut Image, token: &str, fs_layer: &Value) -> Result<(), Box<dyn std::error::Error>> {
    if let Value::String(blob_sum) = &fs_layer["blobSum"] {
        let digest = blob_sum.replace("sha256:", "");
        // let digest = blob_sum.split_off(blob_sum.find(':')?);
        let image_path = get_image_path(image)?;
        let tar_path = format!(
            "{}/{}.tar.gz",
            image_path, digest
        );

        image.fs_layers.push(digest.clone());

        let blob_url = format!(
            "https://registry.hub.docker.com/v2/{}/blobs/{}",
            image.name, blob_sum
        );

        let mut response = reqwest::blocking::Client::new()
            .get(blob_url.as_str())
            .bearer_auth(token)
            .send()?;
        let mut tar_output = File::create(&tar_path)?;
        io::copy(&mut response, &mut tar_output)?;
    } else {
        return Err("blobSum not found".into());
    }

    Ok(())
}

pub fn unpack_image_layers(image: &mut Image) -> Result<(), Box<dyn std::error::Error>> {
    info!("unpacking image layers...");

    for fs_layer in &image.fs_layers {
        let image_path = get_image_path(image)?;
        let layer_path = format!(
            "{}/{}",
            image_path, fs_layer
        );
        let tar_path = format!(
            "{}.tar.gz",
            layer_path
        );
        let tar_gz = File::open(&tar_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);

        if !Path::new(layer_path.as_str()).exists() {
            info!("mkdir {}", layer_path);
            fs::create_dir_all(layer_path.clone())?;
        }

        archive.unpack(layer_path)?;
        info!("unpacked layer {}", fs_layer);
    }

    Ok(())
}

pub fn remove_archives(image: &mut Image) -> Result<(), Box<dyn std::error::Error>> {
    info!("cleaning up image directory...");

    for fs_layer in &image.fs_layers {
        let image_path = get_image_path(image)?;
        let layer_path = format!(
            "{}/{}",
            image_path, fs_layer
        );
        let tar_path = format!(
            "{}.tar.gz",
            layer_path
        );

        fs::remove_file(tar_path)?;
        info!("removed archive layer {}", fs_layer);
    }

    Ok(())
}

pub fn pull(image: &mut Image) -> Result<(), Box<dyn std::error::Error>> {
    let authentication_url = format!(
        "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
        image.name
    );
    let token = get_authentication_token(authentication_url.as_str())?;

    let manifests_url = format!(
        "https://registry.hub.docker.com/v2/{}/manifests/{}",
        image.name, image.reference
    );
    let fs_layers = get_filesystem_layers(token.as_str(), manifests_url.as_str())?;

    let image_path = get_image_path(image)?;
    info!("creating image directory...");
    fs::create_dir_all(image_path)?;

    let number_of_layers = fs_layers.len();
    for (index, fs_layer) in fs_layers.iter().enumerate() {
        info!("downloading layer {} out of {}...", index + 1, number_of_layers);
        download_layer(image, token.as_str(), &fs_layer).expect("download failed");
    }

    unpack_image_layers(image)?;

    remove_archives(image)?;

    Ok(())
}
