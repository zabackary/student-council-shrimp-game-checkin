use std::fmt::{Debug, Display};

use image::RgbaImage;

pub mod mock;

pub trait ServerBackend: Clone {
    type Error: Debug + Display + Send;

    fn initialize() -> Result<(), Self::Error> {
        Ok(())
    }

    async fn upload_photos(photos: Vec<RgbaImage>) -> Result<(), Self::Error>;
}
