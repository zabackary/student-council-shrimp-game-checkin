mod border_radius;

use iced::border::Radius;
use iced::widget::image::Handle;
use iced::Task;
use image::RgbaImage;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub enum CameraMessage {
    CaptureFrame,
    NewFrame(Handle),
}

/// Camera feed.
#[derive(Debug, Clone)]
pub struct CameraFeed<C: crate::backend::cameras::CameraBackendCamera + 'static> {
    camera: Arc<Mutex<C>>,
    current_frame: Arc<Mutex<Option<Handle>>>,
    options: CameraFeedOptions,
}

#[derive(Debug, Clone, Copy)]
pub struct CameraFeedOptions {
    pub radius: Radius,
    pub mirror: bool,
    pub aspect_ratio: Option<f32>,
    pub blur: f32,
}

impl Default for CameraFeedOptions {
    fn default() -> Self {
        Self {
            radius: Radius::from(0),
            mirror: false,
            aspect_ratio: None,
            blur: 0.0,
        }
    }
}

impl<C: crate::backend::cameras::CameraBackendCamera + 'static> CameraFeed<C> {
    pub fn new(camera: C, options: CameraFeedOptions) -> (Self, Task<CameraMessage>) {
        (
            CameraFeed {
                camera: Arc::new(Mutex::new(camera)),
                current_frame: Arc::new(Mutex::new(None)),
                options,
            },
            Task::done(CameraMessage::CaptureFrame),
        )
    }

    pub fn options(&self) -> CameraFeedOptions {
        self.options
    }

    pub fn update_options(&mut self, options: CameraFeedOptions) {
        self.options = options;
    }

    /// Take an image outside of the normal video capture cycle
    pub async fn capture_still(
        &mut self,
        postprocessing_options: CameraFeedOptions,
    ) -> Result<RgbaImage, C::Error> {
        let cloned_camera = self.camera.clone();
        let frame = tokio::task::spawn_blocking(move || {
            cloned_camera
                .lock()
                .expect("failed to lock camera mutex")
                .capture_still_frame()
                .map(|x| image_postprocessing(x, postprocessing_options))
        })
        .await
        .expect("capture_still task terminated unexpectedly")?;
        Ok(image_postprocessing(frame, postprocessing_options))
    }

    /// Take an image outside of the normal video capture cycle
    pub fn capture_still_sync(
        &mut self,
        postprocessing_options: CameraFeedOptions,
    ) -> Result<RgbaImage, C::Error> {
        let frame = self
            .camera
            .lock()
            .expect("failed to lock camera mutex")
            .capture_still_frame()
            .map(|x| image_postprocessing(x, postprocessing_options))?;
        Ok(image_postprocessing(frame, postprocessing_options))
    }

    pub fn update(&mut self, message: CameraMessage) -> Task<CameraMessage> {
        match message {
            CameraMessage::CaptureFrame => {
                let cloned_camera = self.camera.clone();
                let options = self.options;
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let frame = cloned_camera
                                .lock()
                                .expect("failed to lock camera mutex")
                                .capture_video_frame()
                                .expect("failed to capture a video frame");

                            let frame = image_postprocessing(frame, options);

                            // output a handle
                            Handle::from_rgba(frame.width(), frame.height(), frame.into_raw())
                        })
                        .await
                        .unwrap()
                    },
                    CameraMessage::NewFrame,
                )
            }
            CameraMessage::NewFrame(data) => {
                *self.current_frame.lock().expect("failed to lock frame") = Some(data);
                Task::perform(async {}, |_| CameraMessage::CaptureFrame)
            }
        }
    }

    /// Get the image handle of the current frame.
    pub fn handle(&self) -> Handle {
        self.current_frame
            .lock()
            .expect("failed to lock frame")
            .clone()
            .unwrap_or_else(|| Handle::from_rgba(0, 0, vec![]))
    }

    /// Wrap the output of `frame_image` in an `Image` widget.
    pub fn view(&self) -> iced::widget::image::Image<Handle> {
        iced::widget::Image::new(self.handle())
    }
}

fn image_postprocessing(
    frame: image::ImageBuffer<image::Rgba<u8>, Vec<u8>>,
    options: CameraFeedOptions,
) -> image::ImageBuffer<image::Rgba<u8>, Vec<u8>> {
    // crop the frame to meet the aspect ratio
    let mut frame = if let Some(aspect_ratio) = options.aspect_ratio {
        let frame_aspect_ratio = frame.width() as f32 / frame.height() as f32;
        let new_width;
        let new_height;
        let left_offset;
        let top_offset;
        if aspect_ratio < frame_aspect_ratio {
            // trim off left and right
            new_height = frame.height();
            new_width = (frame.height() as f32 * aspect_ratio) as u32;
            left_offset = (frame.width() - new_width) / 2;
            top_offset = 0;
        } else if aspect_ratio > frame_aspect_ratio {
            // trim off top and bottom
            new_width = frame.width();
            new_height = (frame.width() as f32 / aspect_ratio) as u32;
            top_offset = (frame.height() - new_height) / 2;
            left_offset = 0;
        } else {
            // perfect aspect ratio!
            new_width = frame.width();
            new_height = frame.height();
            top_offset = 0;
            left_offset = 0;
        }
        image::imageops::crop_imm(&frame, left_offset, top_offset, new_width, new_height).to_image()
    // this might be pricy...
    } else {
        frame
    };

    // mirror the frame
    if options.mirror {
        image::imageops::flip_horizontal_in_place(&mut frame);
    }

    // apply border radius
    border_radius::round(&mut frame, &options.radius);

    // apply blur
    if options.blur > 0.0 {
        frame = image::imageops::thumbnail(
            &frame,
            (frame.width() as f32 / options.blur) as u32,
            (frame.height() as f32 / options.blur) as u32,
        )
        // We could do:
        // frame = image::imageops::blur(&frame, options.blur);
        // but the performance hit is too high for this kind of application
    }
    image::imageops::resize(
        &frame,
        ((frame.width() as f64) / 1.4) as u32,
        ((frame.height() as f64) / 1.4) as u32,
        image::imageops::FilterType::Triangle,
    )
}
