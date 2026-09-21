//! GTK child attachment for either GPUI XCB or Xlib parent identifiers.
//! The foreign parent is borrowed. GTK owns only the new child and its widgets.
use gtk::{gdk, prelude::*};
use raw_window_handle::RawWindowHandle;
use wry::Rect;

pub(super) fn parent_id(handle: RawWindowHandle) -> Result<u64, String> {
    let id = match handle {
        RawWindowHandle::Xlib(handle) => handle.window,
        RawWindowHandle::Xcb(handle) => u64::from(handle.window.get()),
        _ => return Err("The native browser requires an X11 parent window".into()),
    };
    if id == 0 {
        return Err("The native parent window is invalid".into());
    }
    Ok(id)
}

pub(super) struct ChildSurface {
    window: gtk::Window,
    child: gdk::Window,
    // Retain the borrowed wrapper, never close or destroy the GPUI parent.
    _parent: gdkx11::X11Window,
    pub container: gtk::Box,
}
impl ChildSurface {
    pub fn new(id: u64) -> Result<Self, String> {
        let display = gdk::Display::default()
            .and_then(|display| display.downcast::<gdkx11::X11Display>().ok())
            .ok_or("GTK could not open the X11 display")?;
        let parent = gdkx11::X11Window::foreign_new_for_display(&display, id);
        let child = gdk::Window::new(
            Some(parent.upcast_ref()),
            &gdk::WindowAttr {
                window_type: gdk::WindowType::Child,
                x: Some(0),
                y: Some(0),
                width: 1,
                height: 1,
                ..Default::default()
            },
        );
        if !child.ensure_native() {
            return Err("GTK could not create a native X11 child surface".into());
        }
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_decorated(false);
        window.set_default_size(1, 1);
        let owned_child = child.clone();
        window.connect_realize(move |window| window.set_window(owned_child.clone()));
        window.set_has_window(true);
        window.realize();
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        window.add(&container);
        container.show();
        Ok(Self {
            window,
            child,
            _parent: parent,
            container,
        })
    }
    pub fn viewport(&self, bounds: Option<Rect>) {
        let Some(bounds) = bounds else {
            self.child.hide();
            self.window.hide();
            return;
        };
        let scale = f64::from(self.window.scale_factor().max(1));
        let position = bounds.position.to_logical::<i32>(scale);
        let size = bounds.size.to_logical::<i32>(scale);
        let width = size.width.max(1);
        let height = size.height.max(1);
        self.window.resize(width, height);
        self.window
            .size_allocate(&gtk::Allocation::new(0, 0, width, height));
        self.child
            .move_resize(position.x, position.y, width, height);
        self.window.show();
        // This is a child, not a window-manager-owned GTK toplevel.
        // Map it explicitly as well as its widget hierarchy.
        self.child.show();
    }
}
impl Drop for ChildSurface {
    fn drop(&mut self) {
        self.child.hide();
        self.window.hide();
        self.window.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_xcb_and_xlib_without_borrowing_a_forged_handle() {
        use raw_window_handle::{XcbWindowHandle, XlibWindowHandle};
        use std::num::NonZeroU32;
        assert_eq!(
            parent_id(RawWindowHandle::Xcb(XcbWindowHandle::new(
                NonZeroU32::new(123).unwrap()
            )))
            .unwrap(),
            123
        );
        assert_eq!(
            parent_id(RawWindowHandle::Xlib(XlibWindowHandle::new(123))).unwrap(),
            123
        );
        assert!(parent_id(RawWindowHandle::Xlib(XlibWindowHandle::new(0))).is_err());
    }
}
