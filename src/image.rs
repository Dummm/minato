use std::path::{Path, PathBuf};

use crate::utils;


pub struct Image {
    pub name: String,
    pub reference: String,
    pub fs_layers: Vec<String>,
}

// TODO: Control better how layers are added (load automatically)
// TODO: Move load, add 'exists' function
impl Image {
    pub fn new(identifiers: &str) -> Image {
        let (image_name, image_reference) = utils::split_image_id(identifiers).unwrap();

        Image {
            name: image_name.to_string(),
            reference: image_reference.to_string(),
            fs_layers: Vec::<String>::new(),
        }
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