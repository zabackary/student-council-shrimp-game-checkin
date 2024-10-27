use nokhwa::{
    self,
    pixel_format::RgbAFormat,
    utils::{CameraIndex, CameraInfo, RequestedFormat},
    Camera, NokhwaError,
};

#[derive(Debug, Clone, Copy)]
pub struct NokhwaBackend {}

impl super::CameraBackend for NokhwaBackend {
    type Error = NokhwaError;
    type EnumeratedCamera = CameraInfo;
    type Camera = NokhwaCamera;

    fn initialize() -> Result<(), Self::Error> {
        nokhwa::nokhwa_initialize(|_| {});
        // Lie because it needs to be sync
        Ok(())
    }

    fn enumerate_cameras() -> Result<Vec<nokhwa::utils::CameraInfo>, NokhwaError> {
        if !nokhwa::nokhwa_check() {
            return Err(NokhwaError::UnitializedError);
        }
        nokhwa::query(nokhwa::utils::ApiBackend::Auto)
    }

    fn open_camera(item: Self::EnumeratedCamera) -> Result<NokhwaCamera, Self::Error> {
        Ok(NokhwaCamera::new(item.index().clone()))
    }
}

pub struct NokhwaCamera {
    index: CameraIndex,
    video_camera: Option<Camera>,
    still_camera: Option<Camera>,
}

impl NokhwaCamera {
    pub fn new(index: CameraIndex) -> Self {
        NokhwaCamera {
            index,
            video_camera: None,
            still_camera: None,
        }
    }
}

impl super::CameraBackendCamera for NokhwaCamera {
    type Error = NokhwaError;

    fn capture_still_frame(&mut self) -> Result<image::RgbaImage, NokhwaError> {
        if self.still_camera.is_none() {
            self.video_camera = None; // drop the fast-taking video camera
            let mut camera = Camera::new(
                self.index.clone(),
                RequestedFormat::new::<RgbAFormat>(
                    nokhwa::utils::RequestedFormatType::AbsoluteHighestResolution,
                ),
            )?;
            camera.open_stream()?;
            self.still_camera = Some(camera);
        }
        let camera = self.still_camera.as_mut().unwrap();
        camera.frame()?.decode_image::<RgbAFormat>()
    }

    fn capture_video_frame(&mut self) -> Result<image::RgbaImage, NokhwaError> {
        if self.video_camera.is_none() {
            self.still_camera = None; // drop the high-res still camera
            let mut camera = Camera::new(
                self.index.clone(),
                RequestedFormat::new::<RgbAFormat>(
                    nokhwa::utils::RequestedFormatType::AbsoluteHighestFrameRate,
                ),
            )?;
            camera.open_stream()?;
            self.video_camera = Some(camera);
        }
        let camera = self.video_camera.as_mut().unwrap();
        camera.frame()?.decode_image::<RgbAFormat>()
    }
}
