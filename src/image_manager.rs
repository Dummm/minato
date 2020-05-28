use std::path::Path;

use log::{info, error};
extern crate clap;
use clap::ArgMatches;

use crate::image::Image;


pub struct ImageManager<'a> {
    #[allow(dead_code)]
    image_list: Vec<&'a Image>
}
impl<'a>  ImageManager<'a> {
    pub fn new() -> ImageManager<'a> {
        ImageManager {
            image_list: Vec::new()
        }
    }

    #[allow(dead_code)]
    pub fn pull_with_args(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let image_id = args.value_of("image-id").unwrap();
        self.pull(image_id)
    }
    pub fn pull(&self, image_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("pulling image...");

        // let mut image = match Image::load(image_id).unwrap() {
        //         Some(image) => image,
        //         None => Image::new(image_id)
        //     };
        let mut image = Image::new(image_id);
        info!("image: {} {} {} {}",
            image.id, image.name, image.reference, image.path);

        image.pull()?;
        info!("pulled image.");
        Ok(())
    }

    pub fn list(&self) -> Result<(), Box<dyn std::error::Error>> {
        let home = match dirs::home_dir() {
            Some(path) => path,
            None       => return Err("error getting home directory".into())
        };
        let images_path = format!("{}/.minato/images/json", home.display());
        let images_path = Path::new(&images_path);
        if !images_path.exists() {
            error!("images path not found. exiting...");
            return Ok(());
        };

        // debug!("{}", containers_path.display());
        let images = images_path.read_dir()?;
        let images = images
            .map(|dir|
                format!("{}",
                    dir.unwrap()
                    .path()
                    .file_name().unwrap()
                    .to_str().unwrap()))
            .collect::<Vec<String>>()
            .clone();

        // debug!("{:?}", images);
        println!(
            "{:25} {:25} {:25} {}",
            "id", "name", "reference", "path");
        for i in images {
            let image_name = Path::new(&i).file_stem().unwrap()
                .to_str().unwrap();
            let image_name = image_name.replace("_", "/");
            let image = match Image::load(image_name.as_str()) {
                Ok(i) => i,
                Err(e) => {
                    println!("error: {}", e);
                    continue
                }
            };
            match image {
                Some(img) => {
                    println!(
                        "{:25} {:25} {:25} {}",
                        img.id, img.name, img.reference, img.path);
                },
                None => continue
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn delete_with_args(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let image_id = args.value_of("image-id").unwrap();
        self.delete(image_id)
    }
    pub fn delete(&self, image_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("deleting image...");
        let image = Image::new(image_id);

        image.delete()?;
        info!("deleted image.");
        Ok(())
    }
}
