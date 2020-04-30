use dirs;

use crate::image::Image;
use crate::container::Container;


pub fn get_image_path_with_str(image_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let home = match dirs::home_dir() {
        Some(path) => path,
        None       => return Err("error getting home directory".into())
    };

    Ok(format!(
        "{}/.minato/images/{}",
        home.display(), image_id
    ))
}

pub fn get_image_path(image: &Image) -> Result<String, Box<dyn std::error::Error>> {
    get_image_path_with_str(image.id.as_str())
}

pub fn fix_image_id(image_id: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut id = String::from(image_id);

    if !id.contains(':') {
        id.push_str(":latest");
    }

    if !id.contains('/') {
        id = format!("library/{}", id);
    }

    Ok(id)
}

pub fn split_image_id(image_id: String) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mut ids: Vec<String> = image_id.split(':').map(|str| String::from(str)).collect();
    if ids.len() == 1 {
        ids.push(String::from("latest"));
    }


    if !ids[0].contains('/') {
        ids[0] = format!("library/{}", ids[0]);
    }

    Ok((ids[0].clone(), ids[1].clone()))
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

