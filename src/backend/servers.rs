use std::fmt::{Debug, Display};

use image::RgbaImage;

pub mod server;

pub trait ServerBackend: Clone + Send {
    type Error: Debug + Display + Send;
    type UploadHandle: Debug + Send + Clone;

    fn new() -> Result<Self, Self::Error>;

    fn upload_photo(
        self,
        strip: RgbaImage,
        photos: Vec<RgbaImage>,
    ) -> impl std::future::Future<Output = Result<Self::UploadHandle, Self::Error>> + Send;

    fn send_email(
        self,
        handle: Self::UploadHandle,
        emails: Vec<String>,
    ) -> impl std::future::Future<Output = Result<bool, Self::Error>> + Send;

    fn get_link(self, handle: Self::UploadHandle) -> String;
}

pub type DefaultServerBackend = server::SupabaseBackend;
