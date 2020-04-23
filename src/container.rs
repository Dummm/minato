use std::iter;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

use super::image::Image;

#[derive(Debug, PartialEq)]
pub enum State {
    Creating,
    Created(u32),
    Running(u32),
    Stopped,
}

pub struct Container {
    pub id: String,
    pub image: Option<Image>,
    pub state: State,
}

// TODO: Add methods for container paths
impl Container {
    pub fn new(container_id: Option<&str>, image: Option<Image>) -> Container {
        let id: String = match container_id {
            Some(id) => id.to_string(),
            None => {
                let mut rng = thread_rng();
                iter::repeat(())
                    .map(|()| rng.sample(Alphanumeric))
                    .take(8)
                    .collect::<String>()
            }
        };

        Container {
            id,
            image,
            state: State::Stopped,
        }
    }
}
