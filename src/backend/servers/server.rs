use std::{fmt::Display, io::Cursor};

use dotenv_codegen::dotenv;
use gcp_auth::TokenProvider;
use image::RgbaImage;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    multipart::Part,
    Client,
};
use serde_json::json;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PartialFileMetadata {
    id: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PartialEmailMetadata {
    status: String,
}

impl PartialEmailMetadata {
    fn is_success(&self) -> bool {
        self.status == "success"
    }
}

#[derive(Debug, Clone)]
pub struct UploadHandle {
    pub strip_id: String,
    pub folder_id: String,
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
    type UploadHandle = UploadHandle;

    fn new() -> Result<Self, Self::Error> {
        let client = reqwest::ClientBuilder::new()
            .build()
            .map_err(SupabaseBackendError::Reqwest)?;

        Ok(SupabaseBackend { client })
    }

    /// Uploads a photo to Google Drive and returns the URL of the strip.
    ///
    /// Creates a new folder within the specified folder in Google Drive,
    /// uploads the strip as strip.png, and uploads the individual photos as
    /// photo_1.png, photo_2.png, etc.
    /// Uploads the emails in a newline-separated text file called emails.txt.
    async fn upload_photo(
        self,
        strip: RgbaImage,
        photos: Vec<RgbaImage>,
    ) -> Result<UploadHandle, Self::Error> {
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

        // Create a new folder in Google Drive
        let folder_name = now;
        let folder_metadata = json!({
            "name": folder_name,
            "mimeType": "application/vnd.google-apps.folder",
            "parents": [dotenv!("DRIVE_FOLDER_ID")],
        });
        let folder_response = self
            .client
            .post("https://www.googleapis.com/upload/drive/v3/files")
            .header("Authorization", format!("Bearer {}", token.as_str()))
            .header(
                "Content-Type",
                HeaderValue::from_static("application/json;charset=UTF-8"),
            )
            .body(folder_metadata.to_string())
            .send()
            .await
            .map_err(SupabaseBackendError::Reqwest)?
            .error_for_status()
            .map_err(SupabaseBackendError::Reqwest)?;
        let folder: PartialFileMetadata = folder_response
            .json()
            .await
            .map_err(SupabaseBackendError::Reqwest)?;
        let folder_id = folder.id;

        log::debug!("Uploaded folder");
        log::debug!("Folder ID: {}", folder_id);

        // Upload the strip

        let mut encoded = Vec::new();
        let mut encoded_cursor = Cursor::new(&mut encoded);
        strip
            .write_to(&mut encoded_cursor, image::ImageFormat::Png)
            .map_err(SupabaseBackendError::ImageEncodeDecode)?;
        let file = upload_file(
            encoded,
            "strip.png".to_string(),
            "image/png",
            folder_id.clone(),
            self.client.clone(),
            token.clone(),
        )
        .await?;
        let strip_id = file.id;

        for (i, photo) in photos.iter().enumerate() {
            let mut encoded = Vec::new();
            let mut encoded_cursor = Cursor::new(&mut encoded);
            photo
                .write_to(&mut encoded_cursor, image::ImageFormat::Png)
                .map_err(SupabaseBackendError::ImageEncodeDecode)?;
            upload_file(
                encoded,
                format!("photo_{}.png", i + 1),
                "image/png",
                folder_id.clone(),
                self.client.clone(),
                token.clone(),
            )
            .await?;
        }

        Ok(UploadHandle {
            strip_id,
            folder_id,
        })
    }

    async fn send_email(
        self,
        handle: Self::UploadHandle,
        emails: Vec<String>,
    ) -> Result<bool, Self::Error> {
        let service_account = gcp_auth::CustomServiceAccount::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/service_account_key.json"
        )))
        .map_err(SupabaseBackendError::GcpAuth)?;
        let token = service_account
            .token(&["https://www.googleapis.com/auth/drive"])
            .await
            .map_err(SupabaseBackendError::GcpAuth)?;
        let emails_content = emails.join("\n");
        upload_file(
            emails_content.as_bytes().to_vec(),
            "emails.txt".to_string(),
            "text/plain",
            handle.folder_id.clone(),
            self.client.clone(),
            token.clone(),
        )
        .await?;

        // send a POST request to ENDPOINT_URL with the folderId in JSON in the body
        let endpoint_url = dotenv!("ENDPOINT_URL");
        let body = json!({
            "folderId": handle.folder_id,
        });

        let client = reqwest::Client::new();
        let res = client
            .post(endpoint_url)
            .json(&body)
            .send()
            .await
            .map_err(SupabaseBackendError::Reqwest)?;
        let email_response: PartialEmailMetadata =
            res.json().await.map_err(SupabaseBackendError::Reqwest)?;

        Ok(email_response.is_success())
    }
}

async fn upload_file(
    content: Vec<u8>,
    name: String,
    content_type: &'static str,
    parent_folder_id: String,
    client: Client,
    token: std::sync::Arc<gcp_auth::Token>,
) -> Result<PartialFileMetadata, SupabaseBackendError> {
    log::trace!("Uploading file: {}", name);
    log::trace!("Content type: {}", content_type);
    log::trace!("Parent folder ID: {}", parent_folder_id);
    let mut metadata_headers = HeaderMap::with_capacity(1);
    metadata_headers.append(
        "Content-Type",
        HeaderValue::from_static("application/json;charset=UTF-8"),
    );
    let mut content_headers = HeaderMap::with_capacity(1);
    content_headers.append("Content-Type", HeaderValue::from_static(content_type));
    let form = reqwest::multipart::Form::new()
            .part("", Part::text(json!({
            "parents": [parent_folder_id],
            "name": name,
            "description": format!("Uploaded at {} by photo-booth-v2", chrono::offset::Local::now())
            }).to_string()).headers(metadata_headers))
            .part("", Part::bytes(content).headers(content_headers));
    let request = client
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

    log::debug!("Uploaded file");
    log::debug!("File ID: {}", file.id);

    Ok(file)
}
