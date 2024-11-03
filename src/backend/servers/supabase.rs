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
    ImageEncodeDecode(image::ImageError),
}

impl Display for SupabaseBackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reqwest(err) => write!(f, "reqwest error: {}", err),
            Self::JsonDecode(err) => write!(f, "json decode error: {}", err),
            Self::GcpAuth(err) => write!(f, "service account authorization error: {}", err),
            Self::ImageEncodeDecode(err) => write!(f, "image encode/decode error: {}", err),
        }
    }
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
        self,
        handle: Self::UploadHandle,
    ) -> Result<Vec<RgbaImage>, Self::Error> {
        let join_handles: Vec<_> = self
            .config
            .templates
            .into_iter()
            .map(|template| {
                let request = self
                    .client
                    .get(format!(
                        "{}/functions/v1/{}",
                        dotenv!("SUPABASE_ENDPOINT"),
                        RENDER_TAKE_ENDPOINT
                    ))
                    .query(&[
                        ("takeId", handle.clone()),
                        ("templateId", template.id.clone()),
                    ]);
                tokio::spawn(async move {
                    let img = image::load_from_memory(
                        &request
                            .send()
                            .await
                            .map_err(SupabaseBackendError::Reqwest)?
                            .error_for_status()
                            .map_err(SupabaseBackendError::Reqwest)?
                            .bytes()
                            .await
                            .map_err(SupabaseBackendError::Reqwest)?,
                    )
                    .map_err(SupabaseBackendError::ImageEncodeDecode)?
                    .to_rgba8();
                    Result::<RgbaImage, SupabaseBackendError>::Ok(img)
                })
            })
            .collect();
        let mut results = Vec::with_capacity(join_handles.len());
        for join_handle in join_handles {
            results.push(join_handle.await.expect("future terminated unexpectedly")?);
        }
        Ok(results)
    }

    async fn upload_photos(
        self,
        photos: Vec<RgbaImage>,
    ) -> Result<Self::UploadHandle, Self::Error> {
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
        let join_handles: Vec<_> = photos
            .into_iter()
            .enumerate()
            .map(|(i, photo)| {
                let now = now.clone();
                let token = token.clone();

                let mut encoded = Vec::new();
                let mut encoded_cursor = Cursor::new(&mut encoded);
                photo
                    .write_to(&mut encoded_cursor, image::ImageFormat::Png)
                    .map_err(SupabaseBackendError::ImageEncodeDecode)
                    .expect("could not encode image");

                let name = format!("{now}-frame{i}");
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
                tokio::spawn(async move {
                    let file: PartialFileMetadata = request
                        .send()
                        .await
                        .map_err(SupabaseBackendError::Reqwest)?
                        .error_for_status()
                        .map_err(SupabaseBackendError::Reqwest)?
                        .json()
                        .await
                        .map_err(SupabaseBackendError::Reqwest)?;
                    Ok(format!("https://drive.google.com/uc?id={}", file.id))
                })
            })
            .collect();
        let mut urls = Vec::with_capacity(join_handles.len());
        for join_handle in join_handles {
            urls.push(join_handle.await.expect("future terminated unexpectedly")?);
        }
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
