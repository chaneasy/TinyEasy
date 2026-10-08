//! Original APNG brand assets, decoded once by a background worker.
use crate::error::Error;
use iced::widget::image::Handle;
use image::{AnimationDecoder, codecs::png::PngDecoder};
use std::{
    io::Cursor,
    sync::Arc,
    time::{Duration, Instant},
};

const PANDA: &[u8] = include_bytes!("../assets/panda-waving.png");
const GRASS: &[u8] = include_bytes!("../assets/grass.png");
const ICON_PNGS: [[&[u8]; 4]; 7] = [
    [
        include_bytes!("../assets/icons/clear-light.png"),
        include_bytes!("../assets/icons/clear-dark.png"),
        include_bytes!("../assets/icons/clear-light-disabled.png"),
        include_bytes!("../assets/icons/clear-dark-disabled.png"),
    ],
    [
        include_bytes!("../assets/icons/previous-light.png"),
        include_bytes!("../assets/icons/previous-dark.png"),
        include_bytes!("../assets/icons/previous-light-disabled.png"),
        include_bytes!("../assets/icons/previous-dark-disabled.png"),
    ],
    [
        include_bytes!("../assets/icons/next-light.png"),
        include_bytes!("../assets/icons/next-dark.png"),
        include_bytes!("../assets/icons/next-light-disabled.png"),
        include_bytes!("../assets/icons/next-dark-disabled.png"),
    ],
    [
        include_bytes!("../assets/icons/empty-light.png"),
        include_bytes!("../assets/icons/empty-dark.png"),
        include_bytes!("../assets/icons/empty-light-disabled.png"),
        include_bytes!("../assets/icons/empty-dark-disabled.png"),
    ],
    [
        include_bytes!("../assets/icons/show-light.png"),
        include_bytes!("../assets/icons/show-dark.png"),
        include_bytes!("../assets/icons/show-light-disabled.png"),
        include_bytes!("../assets/icons/show-dark-disabled.png"),
    ],
    [
        include_bytes!("../assets/icons/hide-light.png"),
        include_bytes!("../assets/icons/hide-dark.png"),
        include_bytes!("../assets/icons/hide-light-disabled.png"),
        include_bytes!("../assets/icons/hide-dark-disabled.png"),
    ],
    [
        include_bytes!("../assets/icons/add-light.png"),
        include_bytes!("../assets/icons/add-dark.png"),
        include_bytes!("../assets/icons/add-light-disabled.png"),
        include_bytes!("../assets/icons/add-dark-disabled.png"),
    ],
];

#[derive(Clone, Copy)]
pub enum Icon {
    Clear,
    Previous,
    Next,
    Empty,
    Show,
    Hide,
    Add,
}

#[derive(Clone)]
pub struct Brand {
    pub panda: Arc<Animation>,
    pub grass: Handle,
    icons: Arc<[[Handle; 4]; 7]>,
}
impl Brand {
    pub fn icon(&self, icon: Icon, dark: bool, enabled: bool) -> &Handle {
        let index = match icon {
            Icon::Clear => 0,
            Icon::Previous => 1,
            Icon::Next => 2,
            Icon::Empty => 3,
            Icon::Show => 4,
            Icon::Hide => 5,
            Icon::Add => 6,
        };
        let variant = match (dark, enabled) {
            (false, true) => 0,
            (true, true) => 1,
            (false, false) => 2,
            (true, false) => 3,
        };
        &self.icons[index][variant]
    }
}
pub struct Frame {
    pub handle: Handle,
    pub end: Duration,
}
pub struct Animation {
    pub frames: Vec<Frame>,
    pub width: u32,
    pub height: u32,
    pub duration: Duration,
    pub started: Instant,
}
impl Animation {
    /// Returns the frame and remaining delay using the asset's own timeline.
    pub fn position(&self, elapsed: Duration) -> (usize, Duration) {
        let phase = Duration::from_nanos((elapsed.as_nanos() % self.duration.as_nanos()) as u64);
        let index = self.frames.partition_point(|frame| frame.end <= phase);
        (index, self.frames[index].end - phase)
    }
}
pub fn load() -> Result<Brand, Error> {
    let decoder = PngDecoder::new(Cursor::new(PANDA)).map_err(|_| Error::Image)?;
    let mut frames = Vec::new();
    let mut duration = Duration::ZERO;
    let mut dimensions = None;
    for frame in decoder.apng().map_err(|_| Error::Image)?.into_frames() {
        let frame = frame.map_err(|_| Error::Image)?;
        let (numerator, denominator) = frame.delay().numer_denom_ms();
        let delay =
            Duration::from_micros(u64::from(numerator) * 1000 / u64::from(denominator.max(1)))
                .max(Duration::from_millis(1));
        let rgba = frame.into_buffer();
        let size = rgba.dimensions();
        if dimensions.is_some_and(|old| old != size) {
            return Err(Error::Image);
        }
        dimensions = Some(size);
        duration += delay;
        frames.push(Frame {
            handle: Handle::from_rgba(size.0, size.1, rgba.into_raw()),
            end: duration,
        });
    }
    let (width, height) = dimensions.ok_or(Error::Image)?;
    Ok(Brand {
        panda: Arc::new(Animation {
            frames,
            width,
            height,
            duration,
            started: Instant::now(),
        }),
        grass: static_image(GRASS)?,
        icons: Arc::new([
            icon_images(ICON_PNGS[0])?,
            icon_images(ICON_PNGS[1])?,
            icon_images(ICON_PNGS[2])?,
            icon_images(ICON_PNGS[3])?,
            icon_images(ICON_PNGS[4])?,
            icon_images(ICON_PNGS[5])?,
            icon_images(ICON_PNGS[6])?,
        ]),
    })
}

fn icon_images(files: [&[u8]; 4]) -> Result<[Handle; 4], Error> {
    let [light, dark, light_disabled, dark_disabled] = files;
    Ok([
        static_image(light)?,
        static_image(dark)?,
        static_image(light_disabled)?,
        static_image(dark_disabled)?,
    ])
}

fn static_image(bytes: &[u8]) -> Result<Handle, Error> {
    let rgba = image::load_from_memory(bytes)
        .map_err(|_| Error::Image)?
        .into_rgba8();
    Ok(Handle::from_rgba(
        rgba.width(),
        rgba.height(),
        rgba.into_raw(),
    ))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_logo_decodes_all_frames_and_preserves_loop_timing() {
        let brand = load().unwrap();
        let animation = brand.panda;
        assert_eq!(animation.frames.len(), 20);
        assert_eq!((animation.width, animation.height), (278, 248));
        assert_eq!(animation.duration, Duration::from_millis(800));
        assert_eq!(
            animation.position(Duration::ZERO),
            (0, Duration::from_millis(40))
        );
        assert_eq!(animation.position(Duration::from_millis(40)).0, 1);
        assert_eq!(animation.position(Duration::from_millis(799)).0, 19);
        assert_eq!(animation.position(Duration::from_millis(800)).0, 0);
        assert_eq!(animation.position(Duration::from_millis(1200)).0, 10);
        fn pixels(frame: &Frame) -> &[u8] {
            match &frame.handle {
                Handle::Rgba { pixels, .. } => pixels.as_ref(),
                _ => panic!("expected decoded frame"),
            }
        }
        assert!(
            animation
                .frames
                .iter()
                .skip(1)
                .any(|f| pixels(f) != pixels(&animation.frames[0]))
        );
    }
}
