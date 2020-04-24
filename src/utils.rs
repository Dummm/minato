use dirs;

use crate::image::Image;
use crate::container::Container;


pub fn get_image_path_with_str(image_name: &str, image_reference: &str) -> Result<String, Box<dyn std::error::Error>> {
    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };

    Ok(format!(
        "{}/.minato/images/{}:{}",
        home.display(), image_name, image_reference
    ))
}

pub fn get_image_path(image: &Image) -> Result<String, Box<dyn std::error::Error>> {
    get_image_path_with_str(image.name.as_str(), image.reference.as_str())
}

pub fn get_container_path_with_str(container_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };

    Ok(format!(
        "{}/.minato/containers/{}",
        home.display(), container_id
    ))
}

pub fn get_container_path(container: &Container) -> Result<String, Box<dyn std::error::Error>> {
    get_container_path_with_str(container.id.as_str())
}

pub fn split_image_id(image_id: &str) -> Result<(&str, &str), Box<dyn std::error::Error>> {
    let mut ids: Vec<&str> = image_id.split(':').collect();
    if ids.len() == 1 {
        ids.push("latest");
    }

    Ok((ids[0], ids[1]))
}

// TODO: Clean-up mess
pub fn load_image(image_id: &str) -> Result<Option<Image>, Box<dyn std::error::Error>> {
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