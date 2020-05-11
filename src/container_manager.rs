use clap::ArgMatches;

use log::info;

use crate::image::Image;
use crate::container::Container;

pub struct ContainerManager<'a> {
    #[allow(dead_code)]
    container_list: Vec<&'a Container>
}
impl<'a> ContainerManager<'a> {
    pub fn new() -> ContainerManager<'a> {
        ContainerManager {
            container_list: Vec::new()
        }
    }

    #[allow(dead_code)]
    pub fn create_with_args(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let image_name = args.value_of("image-id").unwrap();
        let container_name = args.value_of("container-name").unwrap();
        self.create(container_name, image_name)
    }
    pub fn create(&self, container_name: &str, image_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let image = match Image::load(image_id)? {
            Some(image) => image,
            None        => {
                // TODO: Add verification
                info!("image not found. exiting...");
                // info!("image not found. trying to pull image...");
                // image_manager::pull(image_id)?;
                // create(container_name, image_id)?;
                return Ok(())
            }
        };

        let container = Container::new(Some(container_name), Some(image));

        container.create()
    }

    // TODO: Fix unwrap here
    #[allow(dead_code)]
    pub fn run_with_args(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let container_name = args.value_of("container-name").unwrap();
        self.run(container_name)
    }
    pub fn run(&self, container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("running container '{}'...", container_name);

        info!("loading container...");
        let container = match Container::load(container_name).unwrap() {
            Some(container) => container,
            None            => {
                info!("container not found. exiting...");
                return Ok(())
            }
        };

        container.run()
    }

    #[allow(dead_code)]
    pub fn delete_with_args(&self, args: &ArgMatches) -> Result<(), Box<dyn std::error::Error>> {
        let container_name = args.value_of("container-name").unwrap();
        self.delete(container_name)
    }
    // TODO: Add contianer state check
    pub fn delete(&self, container_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let container = Container::new(Some(container_name), None);

        container.delete()
    }
}
