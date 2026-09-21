//! A separately owned X11 child attached to either a GPUI XCB or Xlib parent.
//! Foreign-window GTK realization supplies the toplevel frame clock WebKit needs.
//! All X11 and GTK calls use safe public APIs; the GPUI parent is never destroyed.
use gtk::{gdk, prelude::*};
use raw_window_handle::RawWindowHandle;
use std::cell::Cell;
use wry::Rect;
use x11rb::{connection::Connection, protocol::xproto::*, rust_connection::RustConnection};

pub(super) fn parent_id(handle: RawWindowHandle) -> Result<u32, String> {
    let id = match handle {
        RawWindowHandle::Xlib(handle) => u32::try_from(handle.window)
            .map_err(|_| "The native parent identifier is outside the X11 range")?,
        RawWindowHandle::Xcb(handle) => handle.window.get(),
        _ => return Err("The native browser requires an X11 parent window".into()),
    };
    if id == 0 {
        return Err("The native parent window is invalid".into());
    }
    Ok(id)
}

pub(super) struct ChildSurface {
    window: gtk::Window,
    child: u32,
    connection: RustConnection,
    layout: Cell<Option<(i32, i32, u32, u32)>>,
    pub container: gtk::Box,
}
impl ChildSurface {
    pub fn new(parent: u32) -> Result<Self, String> {
        let display = gdk::Display::default()
            .and_then(|display| display.downcast::<gdkx11::X11Display>().ok())
            .ok_or("GTK could not open the X11 display")?;
        let (connection, _) =
            x11rb::connect(Some(display.upcast_ref::<gdk::Display>().name().as_str()))
                .map_err(|e| format!("X11 child connection failed: {e}"))?;
        let child = connection.generate_id().map_err(|e| e.to_string())?;
        connection
            .create_window(
                x11rb::COPY_DEPTH_FROM_PARENT,
                child,
                parent,
                0,
                0,
                1,
                1,
                0,
                WindowClass::INPUT_OUTPUT,
                x11rb::COPY_FROM_PARENT,
                &CreateWindowAux::new(),
            )
            .map_err(|e| e.to_string())?
            .check()
            .map_err(|e| format!("X11 child creation failed: {e}"))?;
        // GDK treats an independently owned native child as a foreign toplevel.
        // A GDK-created Child instead inherits an absent GPUI/GDK frame clock.
        let foreign = gdkx11::X11Window::foreign_new_for_display(&display, u64::from(child));
        let window = gtk::Window::new(gtk::WindowType::Toplevel);
        window.set_decorated(false);
        window.set_default_size(1, 1);
        window.connect_realize(move |window| window.set_window(foreign.clone().upcast()));
        window.set_has_window(true);
        window.realize();
        let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
        window.add(&container);
        container.show();
        Ok(Self {
            window,
            child,
            connection,
            layout: Cell::new(None),
            container,
        })
    }
    pub fn viewport(&self, bounds: Option<Rect>) {
        let Some(bounds) = bounds else {
            if self.layout.take().is_some() {
                let _ = self.connection.unmap_window(self.child);
                let _ = self.connection.flush();
                self.window.hide();
            }
            return;
        };
        let scale = f64::from(self.window.scale_factor().max(1));
        let position = bounds.position.to_physical::<i32>(scale);
        let physical = bounds.size.to_physical::<u32>(scale);
        let layout = (
            position.x,
            position.y,
            physical.width.max(1),
            physical.height.max(1),
        );
        if self.layout.get() == Some(layout) {
            return;
        }
        self.layout.set(Some(layout));
        let _ = self.connection.configure_window(
            self.child,
            &ConfigureWindowAux::new()
                .x(layout.0)
                .y(layout.1)
                .width(layout.2)
                .height(layout.3),
        );
        let logical = bounds.size.to_logical::<i32>(scale);
        self.window
            .resize(logical.width.max(1), logical.height.max(1));
        self.window.size_allocate(&gtk::Allocation::new(
            0,
            0,
            logical.width.max(1),
            logical.height.max(1),
        ));
        self.window.show();
        let _ = self.connection.map_window(self.child);
        let _ = self.connection.flush();
    }
}
impl Drop for ChildSurface {
    fn drop(&mut self) {
        self.window.hide();
        self.window.close();
        // Destroy only this owner's child. GTK may already have destroyed it.
        if let Ok(cookie) = self.connection.destroy_window(self.child) {
            let _ = cookie.check();
        }
        let _ = self.connection.flush();
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
