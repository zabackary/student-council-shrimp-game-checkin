use std::fmt::{Debug, Display};

pub mod nokhwa;

pub trait CameraBackend: Clone {
    type Error: Debug + Send;
    type EnumeratedCamera: Debug + Display + PartialEq + Clone + Send;
    type Camera: CameraBackendCamera;

    fn initialize() -> Result<(), Self::Error> {
        Ok(())
    }
    fn enumerate_cameras() -> Result<Vec<Self::EnumeratedCamera>, Self::Error>;
    fn open_camera(item: Self::EnumeratedCamera) -> Result<Self::Camera, Self::Error>;
}

pub trait CameraBackendCamera: Send {
    type Error: Debug + Send + Clone;

    fn capture_video_frame(&mut self) -> Result<image::RgbaImage, Self::Error>;
    fn capture_still_frame(&mut self) -> Result<image::RgbaImage, Self::Error>;
}

pub type DefaultCameraBackend = nokhwa::NokhwaBackend;
