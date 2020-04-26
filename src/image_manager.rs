use std::fs::{self, File};
use std::path::Path;
use std::io;

use log::info;
use reqwest;
use serde_json::{self, Value};
use tar::Archive;
use flate2::read::GzDecoder;

extern crate clap;
use clap::ArgMatches;

use crate::image::Image;
use crate::utils;


fn get_authentication_token(auth_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    info!("sending authentication token request to: {}...", auth_url);

    let response = reqwest::blocking::get(auth_url)?;
    let response_text = response.text()?;
    let body: Value = serde_json::from_str(response_text.as_str())?;
    info!("parsed json successfully");

    let token = match &body["token"] {
        Value::String(t) => t,
        _ => return Err("token retrieval failed".into()),
    };
    info!("retrieved token successfully");

    Ok(token.clone())
}

fn get_image_json(token: &str, manifests_url: &str) -> Result<Value, Box<dyn std::error::Error>> {
    info!("sending manifests request to: {}...", manifests_url);

    let response = reqwest::blocking::Client::new()
        .get(manifests_url)
        .bearer_auth(token)
        .send()?;
    let response_text = response.text()?;
    let body: Value = serde_json::from_str(response_text.as_str())?;

    Ok(body)
}

fn write_image_json(image_id: &str, body: Value) -> Result<(), Box<dyn std::error::Error>> {
    info!("writing image json...");

    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };
    let json_directory_path_str = format!(
        "{}/.minato/images/json",
        home.display()
    );
    let json_directory_path = Path::new(json_directory_path_str.as_str());

    if !json_directory_path.exists() {
        fs::create_dir_all(json_directory_path)?;
    }

    let json_name = format!(
        "{}.json",
        image_id.replace("/", "_")
    );
    let json_path = json_directory_path.join(json_name);

    serde_json::to_writer(&File::create(&json_path)?, &body)?;
    info!("json path: {}", json_path.to_str().unwrap());

    Ok(())
}

fn extract_layers_from_body(body: Value) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
    info!("extracting fs_layers...");

    let fs_layers = match &body["fsLayers"] {
        Value::Array(fs_layers) => fs_layers,
        _ => return Err("filesystem layers retrieval failed".into()),
    };
    info!("extracted fs_layers successfully");

    Ok(fs_layers.clone())
}

fn download_layer(image: &mut Image, token: &str, fs_layer: &Value) -> Result<(), Box<dyn std::error::Error>> {
    if let Value::String(blob_sum) = &fs_layer["blobSum"] {
        let digest = blob_sum.replace("sha256:", "");
        // let digest = blob_sum.split_off(blob_sum.find(':')?);
        let image_path_str = utils::get_image_path(image)?;
        let tar_path = format!(
            "{}/{}.tar.gz",
            image_path_str, digest
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
    info!("downloaded layer successfully");

    Ok(())
}

// TODO: Change the way unpacking is skipped
fn unpack_image_layers(image: &mut Image) -> Result<(), Box<dyn std::error::Error>> {
    info!("unpacking image layers...");

    for fs_layer in &image.fs_layers {
        let image_path_str = utils::get_image_path(image)?;
        let layer_path = format!(
            "{}/{}",
            image_path_str, fs_layer
        );
        let tar_path = format!(
            "{}.tar.gz",
            layer_path
        );
        let tar_gz = File::open(&tar_path)?;
        let tar = GzDecoder::new(tar_gz);
        let mut archive = Archive::new(tar);

        if !Path::new(layer_path.as_str()).exists() {
            fs::create_dir_all(layer_path.clone())?;
            archive.unpack(layer_path)?;
            info!("unpacked layer {}", fs_layer);
        } else {
            info!("layer {} exists, skipping unpack", fs_layer);
        }

    }
    info!("unpacked layers successfully");

    Ok(())
}

// TODO: Check if file exists before removal?
fn remove_archives(image: &mut Image) -> Result<(), Box<dyn std::error::Error>> {
    info!("cleaning up image directory...");

    for fs_layer in &image.fs_layers {
        let image_path_str = utils::get_image_path(image)?;
        let layer_path = format!(
            "{}/{}",
            image_path_str, fs_layer
        );
        let tar_path = format!(
            "{}.tar.gz",
            layer_path
        );

        fs::remove_file(tar_path)?;
        info!("removed archive layer {}", fs_layer);
    }

    info!("cleaned up successfully");
    Ok(())
}

pub fn pull_with_args(args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let image_id = args.value_of("image-id").unwrap();
    pull(image_id)
}

// TODO: Modularize
pub fn pull(image_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("pulling image...");

    info!("image name: {}", image_id);
    let mut image = match Image::load(image_id).unwrap() {
        Some(image) => image,
        None => Image::new(image_id)
    };

    let image_path_str = utils::get_image_path(&image)?;
    if Path::new(image_path_str.as_str()).exists() {
        info!("image exists. skipping pull...");
        return Ok(())
    }

    let authentication_url = format!(
        "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
        image.name
    );
    let token = get_authentication_token(authentication_url.as_str())?;

    let manifests_url = format!(
        "https://registry.hub.docker.com/v2/{}/manifests/{}",
        image.name, image.reference
    );
    let json = get_image_json(token.as_str(), manifests_url.as_str())?;
    write_image_json(image_id, json.clone())?;
    let fs_layers = extract_layers_from_body(json)?;

    info!("creating image directory...");
    fs::create_dir_all(image_path_str)?;

    let number_of_layers = fs_layers.len();
    for (index, fs_layer) in fs_layers.iter().enumerate() {
        info!("downloading layer {} out of {}...", index + 1, number_of_layers);
        download_layer(&mut image, token.as_str(), &fs_layer).expect("download failed");
    }

    unpack_image_layers(&mut image)?;

    remove_archives(&mut image)?;

    Ok(())
}


fn delete_image_json(image_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("deleting image json...");

    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };
    let json_directory_path_str = format!(
        "{}/.minato/images/jsons",
        home.display()
    );
    let json_directory_path = Path::new(json_directory_path_str.as_str());

    let json_name = format!(
        "{}.json",
        image_id.replace("/", "_")
    );
    let json_path = json_directory_path.join(json_name);

    if !json_path.exists() {
        info!("image json not found. skipping...");
        return Ok(())
    }

    fs::remove_file(json_path)?;
    info!("image json deleted succesfully");

    Ok(())
}

fn delete_image_directory(image_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("deleting image directory...");

    let image_path_str = utils::get_image_path_with_str(image_id)?;
    let image_path = Path::new(image_path_str.as_str());

    if !image_path.exists() {
        info!("image not found. skipping deletion...");
        return Ok(())
    }

    fs::remove_dir_all(image_path)?;

    info!("directory deletion successful");

    Ok(())
}

pub fn delete_with_args(args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
    let image_id = args.value_of("image-id").unwrap();
    delete(image_id)
}

pub fn delete(image_id: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("deleting image '{}'...", image_id);

    delete_image_directory(image_id)?;

    delete_image_json(image_id)?;

    info!("deletion successfull");
    Ok(())
}
