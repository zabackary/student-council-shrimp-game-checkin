use image::RgbaImage;

#[derive(Debug, Clone, Copy)]
pub struct MockBackend {}

impl super::ServerBackend for MockBackend {
    type Error = String;

    fn initialize() -> Result<(), Self::Error> {
        Ok(())
    }

    async fn upload_photos(_photos: Vec<RgbaImage>) -> Result<(), Self::Error> {
        todo!()
    }
}
