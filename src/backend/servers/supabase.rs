use std::{fmt::Display, io::Cursor};

use dotenv_codegen::dotenv;
use gcp_auth::TokenProvider;
use image::RgbaImage;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    multipart::Part,
};
use serde_json::json;

const INSTANCE_DATA_ENDPOINT: &str = "instance-data";
const INSERT_TAKE_ENDPOINT: &str = "insert-take";
const RENDER_TAKE_ENDPOINT: &str = "render-take";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PartialFileMetadata {
    id: String,
}

#[derive(Debug, Clone)]
pub struct SupabaseBackend {
    client: reqwest::Client,
    config: super::ServerConfig,
}

#[derive(Debug)]
pub enum SupabaseBackendError {
    Reqwest(reqwest::Error),
    JsonDecode(reqwest::Error),
    GcpAuth(gcp_auth::Error),
    Encode(image::ImageError),
}

impl Display for SupabaseBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reqwest(err) => write!(f, "reqwest error: {}", err),
            Self::JsonDecode(err) => write!(f, "json decode error: {}", err),
            Self::GcpAuth(err) => write!(f, "service account authorization error: {}", err),
            Self::Encode(err) => write!(f, "image encode error: {}", err),
        }
    }
}

async fn upload_file(
    content: &[u8],
    content_mime: &str,
    name: &str,
    folder: &str,
    token: &str,
    client: &reqwest::Client,
) -> reqwest::Result<PartialFileMetadata> {
    let mut metadata_headers = HeaderMap::with_capacity(1);
    metadata_headers.append(
        "Content-Type",
        HeaderValue::from_static("application/json;charset=UTF-8"),
    );
    let mut content_headers = HeaderMap::with_capacity(1);
    content_headers.append(
        "Content-Type",
        HeaderValue::from_str(&content_mime).expect("bad mime"),
    );
    let form = reqwest::multipart::Form::new()
        .part("", Part::text(json!({
            "parents": [folder],
            "name": name,
            "description": format!("Uploaded at {} by photo-booth-v2", chrono::offset::Local::now())
        }).to_string()).headers(metadata_headers))
        // yay, big allocation
        .part("", Part::bytes(content.to_owned()).headers(content_headers));
    println!("posting...");
    client
        .post("https://www.googleapis.com/upload/drive/v3/files")
        .query(&[("uploadType", "multipart")])
        .multipart(form)
        .header(
            "Content-Type",
            HeaderValue::from_static("multipart/related"),
        )
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

impl super::ServerBackend for SupabaseBackend {
    type Error = SupabaseBackendError;
    type UploadHandle = String;

    fn new() -> Result<Self, Self::Error> {
        let client = reqwest::ClientBuilder::new()
            .build()
            .map_err(SupabaseBackendError::Reqwest)?;

        Ok(SupabaseBackend {
            client,
            config: reqwest::blocking::Client::new()
                .get(format!(
                    "{}/functions/v1/{}",
                    dotenv!("SUPABASE_ENDPOINT"),
                    INSTANCE_DATA_ENDPOINT
                ))
                .query(&[("id", dotenv!("PHOTO_BOOTH_INSTANCE_ID"))])
                .send()
                .map_err(SupabaseBackendError::Reqwest)?
                .error_for_status()
                .map_err(SupabaseBackendError::Reqwest)?
                .json()
                .map_err(SupabaseBackendError::JsonDecode)?,
        })
    }

    fn config(&self) -> &super::ServerConfig {
        &self.config
    }

    async fn update(&mut self) -> Result<(), Self::Error> {
        self.config = self
            .client
            .get(format!(
                "{}/functions/v1/{}",
                dotenv!("SUPABASE_ENDPOINT"),
                INSTANCE_DATA_ENDPOINT
            ))
            .send()
            .await
            .map_err(SupabaseBackendError::Reqwest)?
            .error_for_status()
            .map_err(SupabaseBackendError::Reqwest)?
            .json()
            .await
            .map_err(SupabaseBackendError::JsonDecode)?;
        Ok(())
    }

    async fn download_template_previews(
        &self,
        handle: Self::UploadHandle,
    ) -> Result<Vec<RgbaImage>, Self::Error> {
        todo!()
    }

    async fn upload_photos(
        self,
        photos: Vec<RgbaImage>,
    ) -> Result<Self::UploadHandle, Self::Error> {
        println!("preparing upload");
        let service_account = gcp_auth::CustomServiceAccount::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/service_account_key.json"
        )))
        .map_err(SupabaseBackendError::GcpAuth)?;
        println!("loading account token");
        let token = service_account
            .token(&["https://www.googleapis.com/auth/drive"])
            .await
            .map_err(SupabaseBackendError::GcpAuth)?;
        println!("done account");
        let now = chrono::offset::Local::now().to_string();
        let mut urls = Vec::new();
        for (i, photo) in photos.iter().enumerate() {
            let mut encoded = Vec::new();
            let mut encoded_cursor = Cursor::new(&mut encoded);
            photo
                .write_to(&mut encoded_cursor, image::ImageFormat::WebP)
                .map_err(SupabaseBackendError::Encode)?;
            println!("encoded {i}");
            let file = upload_file(
                &encoded,
                "image/webp",
                &format!("{now}-frame{i}"),
                dotenv!("DRIVE_FOLDER_ID"),
                token.as_str(),
                &self.client,
            )
            .await
            .map_err(SupabaseBackendError::Reqwest)?;
            urls.push(format!("https://drive.google.com/uc?id={}", file.id));
            println!("uploaded {i}");
        }
        println!("done photo upload");
        println!(
            "{:?}",
            self.client
                .post(format!(
                    "{}/functions/v1/{}",
                    dotenv!("SUPABASE_ENDPOINT"),
                    INSERT_TAKE_ENDPOINT
                ))
                .query(&[
                    ("rawUrls", urls.join(",")),
                    ("instanceId", self.config.id.clone()),
                ])
                .send()
                .await
                .map_err(SupabaseBackendError::Reqwest)?
                .text()
                .await
        );
        #[derive(Debug, serde::Serialize, serde::Deserialize)]
        struct Id {
            id: String,
        }
        let id: Id = self
            .client
            .post(format!(
                "{}/functions/v1/{}",
                dotenv!("SUPABASE_ENDPOINT"),
                INSERT_TAKE_ENDPOINT
            ))
            .query(&[
                ("rawUrls", urls.join(",")),
                ("instanceId", self.config.id.clone()),
            ])
            .send()
            .await
            .map_err(SupabaseBackendError::Reqwest)?
            .error_for_status()
            .map_err(SupabaseBackendError::Reqwest)?
            .json()
            .await
            .map_err(SupabaseBackendError::Reqwest)?;
        Ok(id.id)
    }
}
