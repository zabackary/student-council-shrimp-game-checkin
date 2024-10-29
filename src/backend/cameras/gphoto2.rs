use std::fmt::Display;

use gphoto2::{list::CameraDescriptor, Camera, Context};

#[derive(Debug, Clone, Copy)]
pub struct GPhoto2Backend {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraDescriptorWrapper(CameraDescriptor);

impl Display for CameraDescriptorWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} on {}", self.0.model, self.0.port)
    }
}

impl super::CameraBackend for GPhoto2Backend {
    type Error = gphoto2::Error;
    type EnumeratedCamera = CameraDescriptorWrapper;
    type Camera = GPhoto2Camera;

    fn initialize() -> Result<(), Self::Error> {
        Ok(())
    }

    fn enumerate_cameras() -> Result<Vec<CameraDescriptorWrapper>, gphoto2::Error> {
        Ok(gphoto2::context::Context::new()?
            .list_cameras()
            .wait()?
            .map(CameraDescriptorWrapper)
            .collect())
    }

    fn open_camera(item: Self::EnumeratedCamera) -> Result<GPhoto2Camera, Self::Error> {
        let context = gphoto2::context::Context::new()?;
        let camera = context.get_camera(&item.0).wait()?;
        Ok(GPhoto2Camera::new(camera, context))
    }
}

pub struct GPhoto2Camera {
    camera: Camera,
    context: Context,
}

impl GPhoto2Camera {
    pub fn new(camera: Camera, context: Context) -> Self {
        GPhoto2Camera { camera, context }
    }
}

#[derive(Debug, Clone)]
pub struct GPhoto2StringError(String);

impl Display for GPhoto2StringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<gphoto2::Error> for GPhoto2StringError {
    fn from(value: gphoto2::Error) -> Self {
        Self(value.to_string())
    }
}

impl super::CameraBackendCamera for GPhoto2Camera {
    type Error = GPhoto2StringError;

    fn capture_still_frame(&mut self) -> Result<image::RgbaImage, GPhoto2StringError> {
        let path = self.camera.capture_image().wait()?;
        let fs = self.camera.fs();
        let img = image::load_from_memory(
            &fs.download(&path.folder(), &path.name())
                .wait()?
                .get_data(&self.context)
                .wait()?,
        )
        .map_err(|err| gphoto2::Error::new(-1, Some(err.to_string())))?;
        Ok(img.to_rgba8())
    }

    fn capture_video_frame(&mut self) -> Result<image::RgbaImage, GPhoto2StringError> {
        let img = image::load_from_memory(
            &self
                .camera
                .capture_preview()
                .wait()?
                .get_data(&self.context)
                .wait()?,
        )
        .map_err(|err| gphoto2::Error::new(-1, Some(err.to_string())))?;
        Ok(img.to_rgba8())
    }
}
