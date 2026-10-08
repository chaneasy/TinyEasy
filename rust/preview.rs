//! Image-only custom surface: clipped rendering, wheel zoom and captured drag events.
use crate::ui::Message;
use iced::advanced::{
    Clipboard, Shell, image,
    layout::{self, Layout},
    mouse, renderer,
    widget::{self, Widget, tree},
};
use iced::{Element, Event, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

#[derive(Debug, Clone, Copy)]
pub struct Transform {
    pub scale: f32,
    pub offset: Vector,
}
impl Default for Transform {
    fn default() -> Self {
        Self {
            scale: 1.,
            offset: Vector::ZERO,
        }
    }
}
impl Transform {
    pub fn zoom(&mut self, factor: f32) {
        self.scale = (self.scale * factor).clamp(0.1, 10.);
    }
}
#[derive(Default)]
struct Drag {
    last: Option<Point>,
    id: Option<image::Id>,
}
pub struct Preview {
    pub handle: image::Handle,
    pub transform: Transform,
}
impl Widget<Message, Theme, Renderer> for Preview {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Drag>()
    }
    fn state(&self) -> tree::State {
        tree::State::new(Drag::default())
    }
    fn diff(&self, tree: &mut widget::Tree) {
        let state = tree.state.downcast_mut::<Drag>();
        if state.id != Some(self.handle.id()) {
            state.last = None;
            state.id = Some(self.handle.id());
        }
    }
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }
    fn layout(
        &mut self,
        _: &mut widget::Tree,
        _: &Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(Length::Fill, Length::Fill, Size::ZERO))
    }
    fn draw(
        &self,
        _: &widget::Tree,
        renderer: &mut Renderer,
        _: &Theme,
        _: &renderer::Style,
        layout: Layout<'_>,
        _: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        use image::Renderer as _;
        use renderer::Renderer as _;
        let bounds = layout.bounds();
        if let Some(size) = renderer.measure_image(&self.handle) {
            let fit = (bounds.width / size.width.max(1) as f32)
                .min(bounds.height / size.height.max(1) as f32)
                .min(1.);
            let size = Size::new(
                size.width as f32 * fit * self.transform.scale,
                size.height as f32 * fit * self.transform.scale,
            );
            let position = Point::new(
                bounds.center_x() - size.width / 2. + self.transform.offset.x,
                bounds.center_y() - size.height / 2. + self.transform.offset.y,
            );
            renderer.with_layer(bounds, |r| {
                r.draw_image(
                    image::Image::new(self.handle.clone()),
                    Rectangle::new(position, size),
                    *viewport,
                )
            });
        }
    }
    fn update(
        &mut self,
        tree: &mut widget::Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _: &Renderer,
        _: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _: &Rectangle,
    ) {
        let drag = tree.state.downcast_mut::<Drag>();
        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta })
                if cursor.is_over(layout.bounds()) =>
            {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 50.,
                };
                shell.publish(Message::Zoom(1.1f32.powf(y.clamp(-5., 5.))));
                shell.capture_event();
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if cursor.is_over(layout.bounds()) =>
            {
                drag.last = cursor.position();
                shell.capture_event();
                shell.request_redraw();
            }
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                if let Some(last) = drag.last.replace(*position) {
                    shell.publish(Message::Pan(*position - last));
                    shell.capture_event();
                } else {
                    drag.last = None;
                }
            }
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                drag.last = None;
                shell.request_redraw();
            }
            Event::Window(iced::window::Event::Unfocused) => {
                drag.last = None;
            }
            _ => {}
        }
    }
    fn mouse_interaction(
        &self,
        tree: &widget::Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _: &Rectangle,
        _: &Renderer,
    ) -> mouse::Interaction {
        if tree.state.downcast_ref::<Drag>().last.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(layout.bounds()) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::None
        }
    }
}
impl From<Preview> for Element<'_, Message> {
    fn from(value: Preview) -> Self {
        Self::new(value)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn zoom_limits_and_reset() {
        let mut t = Transform::default();
        t.zoom(1000.);
        assert_eq!(t.scale, 10.);
        t.zoom(0.00001);
        assert_eq!(t.scale, 0.1);
        t = Transform::default();
        assert_eq!(t.offset, Vector::ZERO);
    }
}
