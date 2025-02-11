use std::fmt::{Debug, Display};

use image::RgbaImage;

pub mod supabase;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Team {
    pub id: i64,
    pub name: String,
    pub signup_email_address: String,
}

pub trait ServerBackend: Clone + Send {
    type Error: Debug + Display + Send;
    type UploadHandle: Debug + Send + Clone;

    fn new() -> Result<Self, Self::Error>;

    fn teams(self) -> impl std::future::Future<Output = Result<Vec<Team>, Self::Error>> + Send;

    fn upload_photo(
        self,
        photo: RgbaImage,
        team_id: i64,
    ) -> impl std::future::Future<Output = Result<Self::UploadHandle, Self::Error>> + Send;
}

pub type DefaultServerBackend = supabase::SupabaseBackend;
