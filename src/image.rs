pub struct Image {
    pub name: String,
    pub reference: String,
    pub fs_layers: Vec<String>,
}

// TODO: Control better how layers are added (load automatically)
impl Image {
    pub fn new(identifiers: &str) -> Image {
        let (image_name, image_reference) = split_image_id(identifiers).unwrap();

        Image {
            name: image_name.to_string(),
            reference: image_reference.to_string(),
            fs_layers: Vec::<String>::new(),
        }
    }
}

pub fn split_image_id(image_id: &str) -> Result<(&str, &str), Box<dyn std::error::Error>> {
    let mut ids: Vec<&str> = image_id.split(':').collect();
    if ids.len() == 1 {
        ids.push("latest");
    }

    Ok((ids[0], ids[1]))
}