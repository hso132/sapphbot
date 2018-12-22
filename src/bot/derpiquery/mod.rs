use bot::*;
use std::io::Write;
use std::sync::mpsc::Sender;
use std::time::Duration;

pub mod data;
use self::data::ImageResponse;
pub struct Derpiquery {
    images: HashSet<String>,
}

impl Derpiquery {
    pub fn new(images: HashSet<String>) -> Derpiquery {
        Derpiquery { images }
    }
    pub fn run(&mut self, sender: Sender<Vec<ImageResponse>>) {
        'updates: loop {
            std::thread::sleep(Duration::from_secs(10));
            match sender.send(self.compute_new_images()) {
                Err(err) => e!(err),
                _ => (),
            };
        }
    }
    fn compute_new_images(&mut self) -> Vec<data::ImageResponse> {
        let images = data::get_images();
        let mut new_images = HashSet::new();
        let mut new_image_ids = HashSet::new();
        for image in images {
            if !self.images.contains(&image.sha512_hash) {
                new_image_ids.insert(image.sha512_hash.clone());
                new_images.insert(image);
            }
        }

        self.images = self.images.union(&new_image_ids).cloned().collect();
        match File::create(IMAGES_PATH) {
            Ok(mut file) => {
                let json = serde_json::to_string(&self.images).unwrap();
                if let Err(err) = file.write_all(&json.as_bytes()) {
                    e!("Could not write to file", IMAGES_PATH, err)
                }
            }
            Err(err) => e!("Could not write to file", IMAGES_PATH, err),
        }
        new_images.into_iter().collect()
    }
}
