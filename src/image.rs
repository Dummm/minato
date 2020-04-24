use crate::utils;

pub struct Image {
    pub name: String,
    pub reference: String,
    pub fs_layers: Vec<String>,
}

// TODO: Control better how layers are added (load automatically)
impl Image {
    pub fn new(identifiers: &str) -> Image {
        let (image_name, image_reference) = utils::split_image_id(identifiers).unwrap();

        Image {
            name: image_name.to_string(),
            reference: image_reference.to_string(),
            fs_layers: Vec::<String>::new(),
        }
    }
}