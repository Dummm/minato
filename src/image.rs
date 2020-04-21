pub struct Image {
    pub name: String,
    pub reference: String,
    pub fs_layers: Vec<String>,
}

// TODO: Control better how layers are added (load automatically)
impl Image {
    pub fn new(identifiers: &str) -> Image {
        let mut ids: Vec<&str> = identifiers.split(':').collect();
        if ids.len() == 1 {
            ids.push("latest");
        }

        Image {
            name: ids[0].to_string(),
            reference: ids[1].to_string(),
            fs_layers: Vec::<String>::new(),
        }
    }
}