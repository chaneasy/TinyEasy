//! Paint-only APNG playback. Frame ticks never publish application messages or rebuild the UI.
use crate::brand::Animation;
use iced::advanced::{
    Clipboard, Layout, Shell, image, layout, mouse, renderer,
    widget::{self, Widget, tree},
};
use iced::{Element, Event, Length, Rectangle, Size, window};
use std::sync::Arc;

pub struct AnimatedLogo {
    animation: Arc<Animation>,
    size: Size<f32>,
    playing: bool,
    horizontal_offset: f32,
}
struct Playback {
    animation: Arc<Animation>,
    frame: usize,
    focused: bool,
}
impl AnimatedLogo {
    pub fn new(animation: &Arc<Animation>, width: f32, height: f32) -> Self {
        Self {
            animation: Arc::clone(animation),
            size: Size::new(width, height),
            playing: true,
            horizontal_offset: 0.,
        }
    }
    pub fn playing(mut self, playing: bool) -> Self {
        self.playing = playing;
        self
    }
    pub fn offset_x(mut self, offset: f32) -> Self {
        self.horizontal_offset = offset;
        self
    }
}
impl<Message, Theme, Renderer> Widget<Message, Theme, Renderer> for AnimatedLogo
where
    Renderer: image::Renderer<Handle = image::Handle>,
{
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Playback>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(Playback {
            animation: Arc::clone(&self.animation),
            frame: 0,
            focused: true,
        })
    }
    fn diff(&self, tree: &mut widget::Tree) {
        let playback = tree.state.downcast_mut::<Playback>();
        if !Arc::ptr_eq(&playback.animation, &self.animation) {
            playback.animation = Arc::clone(&self.animation);
            playback.frame = 0;
        }
    }
    fn size(&self) -> Size<Length> {
        Size::new(
            Length::Fixed(self.size.width),
            Length::Fixed(self.size.height),
        )
    }
    fn layout(
        &mut self,
        _: &mut widget::Tree,
        _: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(self.size.width, self.size.height, self.size))
    }
    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        _: mouse::Cursor,
        _: &Renderer,
        _: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let playback = tree.state.downcast_mut::<Playback>();
        match event {
            Event::Window(window::Event::Unfocused) => playback.focused = false,
            Event::Window(window::Event::Focused) => {
                playback.focused = true;
                if self.playing {
                    shell.request_redraw();
                }
            }
            Event::Window(window::Event::RedrawRequested(now))
                if self.playing
                    && playback.focused
                    && viewport.intersection(&layout.bounds()).is_some() =>
            {
                let elapsed = now.saturating_duration_since(self.animation.started);
                let (frame, remaining) = self.animation.position(elapsed);
                playback.frame = frame;
                shell.request_redraw_at(window::RedrawRequest::At(*now + remaining));
            }
            _ => {}
        }
    }
    fn draw(
        &self,
        tree: &widget::Tree,
        renderer: &mut Renderer,
        _: &Theme,
        _: &renderer::Style,
        layout: Layout<'_>,
        _: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let bounds = layout.bounds();
        let factor = (bounds.width / self.animation.width as f32)
            .min(bounds.height / self.animation.height as f32);
        let size = Size::new(
            self.animation.width as f32 * factor,
            self.animation.height as f32 * factor,
        );
        let image_bounds = Rectangle::new(
            iced::Point::new(
                bounds.center_x() - size.width / 2. + self.horizontal_offset,
                bounds.center_y() - size.height / 2.,
            ),
            size,
        );
        let playback = tree.state.downcast_ref::<Playback>();
        let Some(clip) = image_bounds.intersection(viewport) else {
            return;
        };
        renderer.draw_image(
            image::Image::new(self.animation.frames[playback.frame].handle.clone()),
            image_bounds,
            clip,
        );
    }
}

impl<'a, Message: 'a, Theme: 'a, Renderer> From<AnimatedLogo>
    for Element<'a, Message, Theme, Renderer>
where
    Renderer: image::Renderer<Handle = image::Handle> + 'a,
{
    fn from(value: AnimatedLogo) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brand::Frame;
    use std::time::{Duration, Instant};

    type LogoWidget = dyn Widget<(), iced::Theme, iced::Renderer>;

    fn logo() -> AnimatedLogo {
        let animation = Arc::new(Animation {
            frames: vec![
                Frame {
                    handle: image::Handle::from_rgba(1, 1, vec![0; 4]),
                    end: Duration::from_millis(40),
                },
                Frame {
                    handle: image::Handle::from_rgba(1, 1, vec![255; 4]),
                    end: Duration::from_millis(80),
                },
            ],
            width: 1,
            height: 1,
            duration: Duration::from_millis(80),
            started: Instant::now(),
        });
        AnimatedLogo::new(&animation, 40., 40.)
    }

    fn dispatch(
        logo: &mut AnimatedLogo,
        tree: &mut widget::Tree,
        event: window::Event,
        viewport: Rectangle,
    ) -> window::RedrawRequest {
        let renderer = iced::Renderer::new(iced::Font::DEFAULT, 16.into());
        let node = layout::Node::new(Size::new(40., 40.));
        let mut messages = Vec::new();
        let mut shell = Shell::new(&mut messages);
        (logo as &mut LogoWidget).update(
            tree,
            &Event::Window(event),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &renderer,
            &mut iced::advanced::clipboard::Null,
            &mut shell,
            &viewport,
        );
        assert!(
            shell.is_empty(),
            "Animation must not rebuild the application"
        );
        shell.redraw_request()
    }

    #[test]
    fn paused_logo_does_not_schedule_frames_and_resumes_on_rebuild() {
        let mut logo = logo().playing(false);
        let mut tree = widget::Tree::new(&logo as &LogoWidget);
        let now = logo.animation.started + Duration::from_millis(40);
        assert_eq!(
            dispatch(
                &mut logo,
                &mut tree,
                window::Event::RedrawRequested(now),
                Rectangle::INFINITE
            ),
            window::RedrawRequest::Wait
        );
        logo = logo.playing(true);
        tree.diff(&logo as &LogoWidget);
        assert_eq!(
            dispatch(
                &mut logo,
                &mut tree,
                window::Event::RedrawRequested(now),
                Rectangle::INFINITE
            ),
            window::RedrawRequest::At(now + Duration::from_millis(40))
        );
        assert_eq!(tree.state.downcast_ref::<Playback>().frame, 1);
    }

    #[test]
    fn unfocused_logo_stops_ticks_and_focus_restarts_playback() {
        let mut logo = logo();
        let mut tree = widget::Tree::new(&logo as &LogoWidget);
        dispatch(
            &mut logo,
            &mut tree,
            window::Event::Unfocused,
            Rectangle::INFINITE,
        );
        let now = Instant::now();
        assert_eq!(
            dispatch(
                &mut logo,
                &mut tree,
                window::Event::RedrawRequested(now),
                Rectangle::INFINITE
            ),
            window::RedrawRequest::Wait
        );
        assert_eq!(
            dispatch(
                &mut logo,
                &mut tree,
                window::Event::Focused,
                Rectangle::INFINITE
            ),
            window::RedrawRequest::NextFrame
        );
    }

    #[test]
    fn offscreen_logo_does_not_schedule_frames() {
        let mut logo = logo();
        let mut tree = widget::Tree::new(&logo as &LogoWidget);
        let viewport = Rectangle::new(iced::Point::new(100., 100.), Size::new(10., 10.));
        assert_eq!(
            dispatch(
                &mut logo,
                &mut tree,
                window::Event::RedrawRequested(Instant::now()),
                viewport
            ),
            window::RedrawRequest::Wait
        );
    }
}
