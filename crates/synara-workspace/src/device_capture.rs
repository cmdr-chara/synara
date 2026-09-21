//! Bounded screenshot decoding before native presentation. Captures are memory
//! only and never become chat attachments or agent-visible tools automatically.
use crate::{WorkspaceError, WorkspaceResult};
use image::{ImageFormat, ImageReader, Limits};
use std::io::Cursor;
use synara_runtime::{DeviceFrame, FrameFormat};

pub struct DeviceCapture {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
impl DeviceCapture {
    pub fn decode(png: Vec<u8>) -> WorkspaceResult<Self> {
        if png.len() > 32 * 1024 * 1024 || !png.starts_with(b"\x89PNG\r\n\x1a\n") {
            return Err(WorkspaceError::Invalid(
                "Invalid or oversized device PNG".into(),
            ));
        }
        let mut limits = Limits::default();
        limits.max_image_width = Some(8192);
        limits.max_image_height = Some(8192);
        limits.max_alloc = Some(32 * 1024 * 1024);
        let mut reader = ImageReader::with_format(Cursor::new(&png), ImageFormat::Png);
        reader.limits(limits);
        let image = reader.decode().map_err(|_| {
            WorkspaceError::Invalid(
                "Device capture could not be decoded within the image limits".into(),
            )
        })?;
        let (width, height) = (image.width(), image.height());
        // RGB/paletted images can fit the decoder budget but grow during RGBA
        // conversion. Check the expanded pixel buffer before allocating it.
        if !rgba_fits(width, height) {
            return Err(WorkspaceError::Invalid(
                "Decoded device frame exceeds the RGBA allocation limit".into(),
            ));
        }
        // Reuse the portable device frame invariant rather than trusting encoded
        // header dimensions or adding a second definition of a legal frame.
        DeviceFrame::new(
            width,
            height,
            FrameFormat::Rgba8,
            image.into_rgba8().into_raw(),
            0,
        )?;
        Ok(Self { png, width, height })
    }
    pub fn orientation(&self) -> &'static str {
        if self.width > self.height {
            "Landscape"
        } else if self.height > self.width {
            "Portrait"
        } else {
            "Square"
        }
    }
}

fn rgba_fits(width: u32, height: u32) -> bool {
    width > 0 && height > 0 && u64::from(width) * u64::from(height) <= (32 * 1024 * 1024) / 4
}

/// Pointer coordinates for a Contain viewport. Letterbox margins do not target
/// the device. Resizing changes presentation, never the remote resolution.
pub fn device_viewport_point(
    x: f32,
    y: f32,
    view_width: f32,
    view_height: f32,
    width: u32,
    height: u32,
) -> Option<(u32, u32)> {
    if [x, y, view_width, view_height]
        .iter()
        .any(|n| !n.is_finite())
        || view_width <= 0.
        || view_height <= 0.
        || width == 0
        || height == 0
    {
        return None;
    }
    let scale = (view_width / width as f32).min(view_height / height as f32);
    let left = (view_width - width as f32 * scale) / 2.;
    let top = (view_height - height as f32 * scale) / 2.;
    let (x, y) = ((x - left) / scale, (y - top) / scale);
    (x >= 0. && y >= 0. && x < width as f32 && y < height as f32)
        .then(|| (x.floor() as u32, y.floor() as u32))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_rgba_expansion_before_conversion() {
        assert!(rgba_fits(4096, 2048));
        assert!(!rgba_fits(4096, 2049));
        assert!(!rgba_fits(u32::MAX, u32::MAX));
        assert!(!rgba_fits(0, 100));
    }
    #[test]
    fn rejects_letterbox_edges_nonfinite_and_zero_sized_viewports() {
        assert_eq!(
            device_viewport_point(100., 100., 200., 200., 100, 200),
            Some((50, 100))
        );
        for (x, y) in [
            (0., 100.),
            (200., 100.),
            (150., 100.),
            (100., 200.),
            (f32::NAN, 0.),
        ] {
            assert_eq!(device_viewport_point(x, y, 200., 200., 100, 200), None);
        }
        assert_eq!(device_viewport_point(0., 0., 0., 0., 100, 200), None);
        assert_eq!(
            device_viewport_point(50., 0., 200., 200., 100, 200),
            Some((0, 0))
        );
    }
    #[test]
    fn capture_decode_validates_pixels_and_orientation_not_just_header() {
        let mut png = Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(image::RgbaImage::new(2, 3))
            .write_to(&mut png, ImageFormat::Png)
            .unwrap();
        let capture = DeviceCapture::decode(png.into_inner()).unwrap();
        assert_eq!(
            (capture.width, capture.height, capture.orientation()),
            (2, 3, "Portrait")
        );
        assert!(DeviceCapture::decode(b"\x89PNG\r\n\x1a\n".to_vec()).is_err());
        assert!(DeviceCapture::decode(vec![]).is_err());
    }
}
