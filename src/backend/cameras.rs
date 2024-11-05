use std::fmt::{Debug, Display};

#[cfg(feature = "camera_gphoto2")]
pub mod gphoto2;
#[cfg(feature = "camera_nokhwa")]
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

#[cfg(all(feature = "camera_nokhwa", feature = "camera_gphoto2"))]
compile_error!(
    "feature \"camera_nokhwa\" and feature \"camera_gphoto2\" cannot be enabled at the same time"
);
#[cfg(not(any(feature = "camera_nokhwa", feature = "camera_gphoto2")))]
compile_error!("one of feature \"camera_nokhwa\" and feature \"camera_gphoto2\" should be enabled");

#[cfg(feature = "camera_gphoto2")]
pub type DefaultCameraBackend = gphoto2::GPhoto2Backend;
#[cfg(feature = "camera_nokhwa")]
pub type DefaultCameraBackend = nokhwa::NokhwaBackend;
