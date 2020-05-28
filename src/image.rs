use std::io::copy;
use std::fs::{create_dir_all, File, remove_file, remove_dir_all};
use std::path::Path;

use log::{debug, info};
use reqwest;
use serde_json::{self, Value};
use tar::Archive;
use flate2::read::GzDecoder;
extern crate clap;

use crate::utils;



pub struct Image {
    pub id: String,
    pub name: String,
    pub reference: String,
    pub fs_layers: Vec<String>,
    pub path: String
}
// TODO: Control better how layers are added (load automatically)
// TODO: Move load, add 'exists' function
// TODO: Check to see if 'self's are required
impl Image {
    pub fn new(image_id: &str) -> Image {
        let id = utils::fix_image_id(image_id).unwrap();
        let (image_name, image_reference) = utils::split_image_id(id.clone()).unwrap();

        let path = utils::get_image_path_with_str(id.as_str()).unwrap();

        Image {
            id,
            name: image_name,
            reference: image_reference,
            fs_layers: Vec::<String>::new(),
            path
        }
    }

    // TODO: Clean-up mess
    pub fn load(image_id: &str) -> Result<Option<Image>, Box<dyn std::error::Error>> {
        let mut image = Image::new(image_id);

        let image_path = Path::new(&image.path);
        if !image_path.exists() {
            return Ok(None);
        };

        let layers = image_path.read_dir()?;
        image.fs_layers = layers
            .map(|dir|
                format!("{}",
                    dir.unwrap()
                    .path()
                    .file_name().unwrap()
                    .to_str().unwrap()))
            .collect::<Vec<String>>()
            .clone();

        Ok(Some(image))
    }

    fn get_authentication_token(&self, auth_url: &str) -> Result<String, Box<dyn std::error::Error>> {
        info!("sending authentication token request to: {}...", auth_url);

        let response = reqwest::blocking::get(auth_url)?;
        let response_text = response.text()?;
        let body: Value = serde_json::from_str(response_text.as_str())?;
        info!("parsed json successfully");

        let token = match &body["token"] {
            Value::String(t) => t,
            _ => return Err("token retrieval failed".into()),
        };

        info!("retrieved token.");
        Ok(token.clone())
    }
    fn get_image_json(&self, token: &str, manifests_url: &str) -> Result<Value, Box<dyn std::error::Error>> {
        info!("sending manifests request to: {}...", manifests_url);

        let response = reqwest::blocking::Client::new()
            .get(manifests_url)
            .bearer_auth(token)
            .send()?;
        let response_text = response.text()?;
        let body: Value = serde_json::from_str(response_text.as_str())?;

        info!("retrieved manifests.");
        Ok(body)
    }
    fn write_image_json(&self, body: Value) -> Result<(), Box<dyn std::error::Error>> {
        info!("writing image json...");
        let image_id = utils::fix_image_id(&self.id).unwrap();

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
            create_dir_all(json_directory_path)?;
        }

        let json_name = format!(
            "{}.json",
            image_id.replace("/", "_")
        );
        let json_path = json_directory_path.join(json_name);

        serde_json::to_writer(&File::create(&json_path)?, &body)?;
        debug!("json path: {}", json_path.to_str().unwrap());

        info!("written image json");
        Ok(())
    }
    fn extract_layers_from_body(&self, body: Value) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        info!("extracting fs_layers...");

        let fs_layers = match &body["fsLayers"] {
            Value::Array(fs_layers) => fs_layers,
            _ => return Err("filesystem layers retrieval failed".into()),
        };

        info!("extracted fs_layers.");
        Ok(fs_layers.clone())
    }
    fn download_layer(&mut self, token: &str, fs_layer: &Value) -> Result<(), Box<dyn std::error::Error>> {
        if let Value::String(blob_sum) = &fs_layer["blobSum"] {
            let digest = blob_sum.replace("sha256:", "");
            // let digest = blob_sum.split_off(blob_sum.find(':')?);

            let tar_path = format!(
                "{}/{}.tar.gz",
                &self.path, digest
            );

            self.fs_layers.push(digest.clone());

            let blob_url = format!(
                "https://registry.hub.docker.com/v2/{}/blobs/{}",
                self.name, blob_sum
            );

            let mut response = reqwest::blocking::Client::new()
                .get(blob_url.as_str())
                .bearer_auth(token)
                .send()?;
            let mut tar_output = File::create(&tar_path)?;
            copy(&mut response, &mut tar_output)?;
        } else {
            return Err("blobSum not found".into());
        }

        Ok(())
    }
    // TODO: Change the way unpacking is skipped
    fn unpack_image_layers(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("unpacking image layers...");

        for fs_layer in &self.fs_layers {
            let image_path_str = utils::get_image_path(&self)?;
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
                create_dir_all(layer_path.clone())?;
                archive.unpack(layer_path)?;
                info!("unpacked layer {}", fs_layer);
            } else {
                info!("layer {} exists, skipping unpack", fs_layer);
            }

        }
        info!("unpacked layers.");

        Ok(())
    }
    // TODO: Check if file exists before removal?
    fn remove_archives(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("cleaning up image directory...");

        for fs_layer in &self.fs_layers {
            let layer_path = format!(
                "{}/{}",
                &self.path, fs_layer
            );
            let tar_path = format!(
                "{}.tar.gz",
                layer_path
            );

            if Path::new(&tar_path).exists() {
                remove_file(tar_path)?;
            }
            info!("removed archive layer {}", fs_layer);
        }

        info!("cleaned up image directory.");
        Ok(())
    }
    fn pull_from_docker(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("pulling image from docker repository...");

        let authentication_url = format!(
            "https://auth.docker.io/token?service=registry.docker.io&scope=repository:{}:pull",
            &self.name
        );
        let token = self.get_authentication_token(authentication_url.as_str())?;

        let manifests_url = format!(
            "https://registry.hub.docker.com/v2/{}/manifests/{}",
            &self.name, &self.reference
        );
        let json = self.get_image_json(token.as_str(), manifests_url.as_str())?;
        self.write_image_json(json.clone())?;
        let fs_layers = self.extract_layers_from_body(json)?;

        info!("creating image directory...");
        create_dir_all(&self.path)?;

        let number_of_layers = fs_layers.len();
        for (index, fs_layer) in fs_layers.iter().enumerate() {
            info!("downloading layer {} out of {}...", index + 1, number_of_layers);
            self.download_layer(token.as_str(), &fs_layer)?;
            info!("downloaded layer successfully");
        }

        self.unpack_image_layers()?;

        self.remove_archives()?;

        info!("pulled image from docker repository.");
        Ok(())
    }
    pub fn pull(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("pulling image...");

        if Path::new(&self.path).exists() {
            info!("image exists. skipping pull...");
            return Ok(())
        }

        self.pull_from_docker()?;

        info!("pulled image.");
        Ok(())
    }

    fn delete_image_json(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("deleting image json...");

        let home = match dirs::home_dir() {
            Some(path) => path,
            None       => return Err("error getting home directory".into())
        };
        let json_directory_path_str = format!(
            "{}/.minato/images/json",
            home.display()
        );
        let json_directory_path = Path::new(json_directory_path_str.as_str());

        let json_name = format!(
            "{}.json",
            &self.id.replace("/", "_")
        );
        let json_path = json_directory_path.join(json_name);

        info!("json path: {}", json_path.display());
        if !json_path.exists() {
            info!("image json not found. skipping...");
            return Ok(())
        }

        remove_file(json_path)?;

        info!("deleted image json.");
        Ok(())
    }
    fn delete_image_directory(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("deleting image directory...");

        let image_path = Path::new(&self.path);
        info!("image path: {}", image_path.display());
        if !image_path.exists() {
            info!("image not found. skipping deletion...");
            return Ok(())
        }

        remove_dir_all(image_path)?;

        info!("deleted image directory.");
        Ok(())
    }
    pub fn delete(&self) -> Result<(), Box<dyn std::error::Error>> {
        info!("deleting image...");

        if !Path::new(&self.path).exists() {
            info!("image doesn't exists. skipping pull...");
            return Ok(())
        }

        self.delete_image_json()?;
        self.delete_image_directory()?;

        info!("deleted image.");
        Ok(())
    }

}

