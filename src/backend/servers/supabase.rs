use std::{fmt::Display, io::Cursor};

use dotenv_codegen::dotenv;
use gcp_auth::TokenProvider;
use image::RgbaImage;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    multipart::Part,
};
use serde_json::json;

const TEAMS_ENDPOINT: &str = "team-data";
const UPLOAD_TEAM_MUG: &str = "update-team-mug";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PartialFileMetadata {
    id: String,
}

#[derive(Debug, Clone)]
pub struct SupabaseBackend {
    client: reqwest::Client,
}

#[derive(Debug)]
pub enum SupabaseBackendError {
    Reqwest(reqwest::Error),
    GcpAuth(gcp_auth::Error),
    ImageEncodeDecode(image::ImageError),
}

impl Display for SupabaseBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reqwest(err) => write!(f, "reqwest error: {}", err),
            Self::GcpAuth(err) => write!(f, "service account authorization error: {}", err),
            Self::ImageEncodeDecode(err) => write!(f, "image encode/decode error: {}", err),
        }
    }
}

impl super::ServerBackend for SupabaseBackend {
    type Error = SupabaseBackendError;
    type UploadHandle = ();

    fn new() -> Result<Self, Self::Error> {
        let client = reqwest::ClientBuilder::new()
            .build()
            .map_err(SupabaseBackendError::Reqwest)?;

        Ok(SupabaseBackend { client })
    }

    async fn teams(self) -> Result<Vec<super::Team>, Self::Error> {
        let response = self
            .client
            .get(format!(
                "{}/functions/v1/{}",
                dotenv!("SUPABASE_ENDPOINT"),
                TEAMS_ENDPOINT
            ))
            .send()
            .await
            .map_err(SupabaseBackendError::Reqwest)?
            .error_for_status()
            .map_err(SupabaseBackendError::Reqwest)?
            .json::<Vec<super::Team>>()
            .await
            .map_err(SupabaseBackendError::Reqwest)?;
        Ok(response)
    }

    async fn upload_photo(self, photo: RgbaImage, team_id: i64) -> Result<(), Self::Error> {
        let service_account = gcp_auth::CustomServiceAccount::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/service_account_key.json"
        )))
        .map_err(SupabaseBackendError::GcpAuth)?;
        let token = service_account
            .token(&["https://www.googleapis.com/auth/drive"])
            .await
            .map_err(SupabaseBackendError::GcpAuth)?;
        let now = chrono::offset::Local::now().to_string();

        let mut encoded = Vec::new();
        let mut encoded_cursor = Cursor::new(&mut encoded);
        photo
            .write_to(&mut encoded_cursor, image::ImageFormat::Png)
            .map_err(SupabaseBackendError::ImageEncodeDecode)?;

        let name = format!("{now}-frame");
        let mut metadata_headers = HeaderMap::with_capacity(1);
        metadata_headers.append(
            "Content-Type",
            HeaderValue::from_static("application/json;charset=UTF-8"),
        );
        let mut content_headers = HeaderMap::with_capacity(1);
        content_headers.append("Content-Type", HeaderValue::from_static("image/webp"));
        let form = reqwest::multipart::Form::new()
            .part("", Part::text(json!({
            "parents": [dotenv!("DRIVE_FOLDER_ID")],
            "name": name,
            "description": format!("Uploaded at {} by photo-booth-v2", chrono::offset::Local::now())
            }).to_string()).headers(metadata_headers))
            .part("", Part::bytes(encoded).headers(content_headers));
        let request = self
            .client
            .post("https://www.googleapis.com/upload/drive/v3/files")
            .query(&[("uploadType", "multipart")])
            .multipart(form)
            .header(
                "Content-Type",
                HeaderValue::from_static("multipart/related"),
            )
            .header("Authorization", format!("Bearer {}", token.as_str()));

        let file: PartialFileMetadata = request
            .send()
            .await
            .map_err(SupabaseBackendError::Reqwest)?
            .error_for_status()
            .map_err(SupabaseBackendError::Reqwest)?
            .json()
            .await
            .map_err(SupabaseBackendError::Reqwest)?;
        let url = format!("https://drive.google.com/uc?id={}", file.id);

        self.client
            .post(format!(
                "{}/functions/v1/{}",
                dotenv!("SUPABASE_ENDPOINT"),
                UPLOAD_TEAM_MUG
            ))
            .query(&[("mugUrl", url), ("id", team_id.to_string())])
            .send()
            .await
            .map_err(SupabaseBackendError::Reqwest)?
            .error_for_status()
            .map_err(SupabaseBackendError::Reqwest)?;
        Ok(())
    }
}
