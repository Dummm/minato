use log::info;
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
    // TODO: Modularize
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
