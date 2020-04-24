use std::path::{Path, PathBuf};

use crate::utils;


// TODO: Save/load container structures in jsons (serde_json)
pub struct Image {
    pub name: String,
    pub reference: String,
    pub fs_layers: Vec<String>,
}

// TODO: Control better how layers are added (load automatically)
// TODO: Move load, add 'exists' function
impl Image {
    pub fn new(image_id: &str) -> Image {
        let (image_name, image_reference) = utils::split_image_id(image_id).unwrap();

        Image {
            name: image_name.to_string(),
            reference: image_reference.to_string(),
            fs_layers: Vec::<String>::new(),
        }
    }

    // TODO: Clean-up mess
    pub fn load(image_id: &str) -> Result<Option<Image>, Box<dyn std::error::Error>> {
        let mut image = Image::new(image_id);

        let image_path = match image.get_path().unwrap() {
            Some(path) => path,
            None       => return Ok(None)
        };

        let layers = image_path.read_dir()?;
        image.fs_layers = layers
            .map(|dir| format!("{}",
                dir.unwrap()
                .path()
                .file_name().unwrap()
                .to_str().unwrap()))
            .collect::<Vec<String>>()
            .clone();

        Ok(Some(image))
    }

    pub fn get_path(&self) -> Result<Option<PathBuf>, Box<dyn std::error::Error>>{
        let image_path_str = utils::get_image_path(&self)?;
        let image_path = Path::new(image_path_str.as_str());

        if !image_path.exists() {
            return Ok(None)
        }

        Ok(Some(PathBuf::from(image_path)))
    }
}