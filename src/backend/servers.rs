use std::fmt::{Debug, Display};

use image::RgbaImage;

pub mod supabase;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ServerConfig {
    id: String,
    name: String,
    paid_information: String,
    paid_information_alt: String,
    contact_name: String,
    contact_email: String,
    paid_is_unlocked: Option<bool>,
}

pub trait ServerBackend: Clone + Send {
    type Error: Debug + Display + Send;
    type UploadHandle: Debug + Send + Clone;

    fn new() -> Result<Self, Self::Error>;

    async fn update(&mut self) -> Result<(), Self::Error>;

    fn config(&self) -> &ServerConfig;

    fn download_template_previews(
        &self,
        handle: Self::UploadHandle,
    ) -> impl std::future::Future<Output = Result<Vec<RgbaImage>, Self::Error>> + Send;

    fn upload_photos(
        self,
        photos: Vec<RgbaImage>,
    ) -> impl std::future::Future<Output = Result<Self::UploadHandle, Self::Error>> + Send;
}

pub type DefaultServerBackend = supabase::SupabaseBackend;
