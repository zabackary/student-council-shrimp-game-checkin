use std::fmt::{Debug, Display};

use image::RgbaImage;

pub mod supabase;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ServerTemplate {
    id: String,
    name: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ServerConfig {
    pub id: String,
    pub name: String,
    pub paid_information: String,
    pub paid_information_alt: String,
    pub contact_name: String,
    pub contact_email: String,
    pub paid_is_unlocked: Option<bool>,
    pub templates: Vec<ServerTemplate>,
}

pub trait ServerBackend: Clone + Send {
    type Error: Debug + Display + Send;
    type UploadHandle: Debug + Send + Clone;

    fn new() -> Result<Self, Self::Error>;

    fn is_unlocked(
        self,
    ) -> impl std::future::Future<Output = Result<Option<bool>, Self::Error>> + Send;

    fn config(&self) -> &ServerConfig;

    fn download_template_previews(
        self,
        handle: Self::UploadHandle,
    ) -> impl std::future::Future<Output = Result<Vec<RgbaImage>, Self::Error>> + Send;

    fn upload_photos(
        self,
        photos: Vec<RgbaImage>,
    ) -> impl std::future::Future<Output = Result<Self::UploadHandle, Self::Error>> + Send;
}

pub type DefaultServerBackend = supabase::SupabaseBackend;
