use smithay_client_toolkit::{
    reexports::{
        client::protocol::{wl_compositor, wl_output, wl_shm, wl_surface},
        client::{Attached, Main},
        protocols::wlr::unstable::layer_shell::v1::client::{
            zwlr_layer_shell_v1, zwlr_layer_surface_v1,
        },
    },
    shm::DoubleMemPool,
};

use fontdue::layout::*;

use crate::lock::canvas::Canvas;
use chrono::Timelike;
use fontdue::Font;
use std::cell::Cell;
use std::cmp::min;
use std::rc::Rc;
use std::{error, fmt, io};

#[derive(PartialEq, Copy, Clone)]
enum RenderEvent {
    Configure { width: u32, height: u32 },
    Close,
}

#[derive(Debug)]
enum DrawError {
    NoFreePool,
    Io(io::Error),
}

impl From<io::Error> for DrawError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl error::Error for DrawError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::NoFreePool => None,
            Self::Io(err) => err.source(),
        }
    }
}

impl fmt::Display for DrawError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFreePool => write!(f, "No free shm pool for drawing"),
            Self::Io(err) => write!(f, "I/O error while drawing: {}", err),
        }
    }
}

pub struct LockSurface {
    surface: Main<wl_surface::WlSurface>,
    layer_surface: Main<zwlr_layer_surface_v1::ZwlrLayerSurfaceV1>,
    next_render_event: Rc<Cell<Option<RenderEvent>>>,
    pools: DoubleMemPool,
    dimensions: (usize, usize),
    redraw: bool,
    color: u32,
    chars_entered: u32,
    fonts: [Font; 1],
}

impl LockSurface {
    pub fn new(
        output: &wl_output::WlOutput,
        compositor: &Attached<wl_compositor::WlCompositor>,
        layer_shell: &Attached<zwlr_layer_shell_v1::ZwlrLayerShellV1>,
        shm: Attached<wl_shm::WlShm>,
        color: u32,
    ) -> Self {
        let surface = compositor.create_surface();
        // We don't currently care about dpi awareness, but that may need to change eventually
        surface.quick_assign(|_, _, _| {});

        // Mark the entire surface as opaque. This isn't strictly required, but serves as an
        // optimization hit for the compositor
        let region = compositor.create_region();
        region.add(0, 0, i32::MAX, i32::MAX);
        surface.set_opaque_region(Some(&region));
        region.destroy();

        let layer_surface = layer_shell.get_layer_surface(
            &surface,
            Some(output),
            zwlr_layer_shell_v1::Layer::Overlay,
            "lockscreen".to_owned(),
        );

        // Size of 0,0 indicates that the server should decide the size
        layer_surface.set_size(0, 0);
        // Anchor to all edges of the output, filling it entirely
        layer_surface.set_anchor(zwlr_layer_surface_v1::Anchor::all());
        layer_surface.set_exclusive_zone(-1);
        layer_surface.set_keyboard_interactivity(1);

        let next_render_event = Rc::new(Cell::new(None::<RenderEvent>));
        let next_render_event_handle = Rc::clone(&next_render_event);
        layer_surface.quick_assign(move |layer_surface, event, _| {
            match (event, next_render_event_handle.get()) {
                (zwlr_layer_surface_v1::Event::Closed, _) => {
                    next_render_event_handle.set(Some(RenderEvent::Close));
                }
                (zwlr_layer_surface_v1::Event::Configure { serial, width, height }, next)
                    if next != Some(RenderEvent::Close) =>
                {
                    layer_surface.ack_configure(serial);
                    next_render_event_handle.set(Some(RenderEvent::Configure { width, height }));
                }
                (_, _) => {}
            }
        });

        // Commit so that the server will send a configure event
        surface.commit();

        // TODO: this callback should technically trigger a redraw, however it is currently very
        // unlikely to be reached
        let pools = DoubleMemPool::new(shm, |_| {}).unwrap_or_else(|err| {
            log::error!("Failed to create shm pools: {}", err);
            panic!();
        });

        let font =
            include_bytes!("/home/will/.fonts/fonts/ttf/JetBrainsMono-Regular.ttf") as &[u8];
        // Parse it into the font type.
        let fonts = [fontdue::Font::from_bytes(font, fontdue::FontSettings::default()).unwrap()];

        Self {
            surface,
            layer_surface,
            next_render_event,
            pools,
            dimensions: (0, 0),
            redraw: false,
            color,
            chars_entered: 0,
            fonts,
        }
    }

    /// Set the color of the surface. Will not take effect until handle_events() is called.
    pub fn set_color(&mut self, color: u32) {
        self.color = color;
        self.redraw = true
    }

    pub fn chars_entered(&mut self, num: u32) {
        self.chars_entered = num;
    }

    pub fn set_redraw(&mut self) {
        self.redraw = true
    }

    /// Handles any events that have occurred since the last call, redrawing if needed.
    /// Returns true if the surface should be dropped.
    pub fn handle_events(&mut self) -> bool {
        match self.next_render_event.take() {
            Some(RenderEvent::Close) => return true,
            Some(RenderEvent::Configure { width, height }) => {
                self.dimensions = (width as usize, height as usize);
                self.redraw = true;
            }
            None => {}
        }

        if self.redraw {
            match self.redraw() {
                Ok(()) => self.redraw = false,
                Err(err) => log::error!("{}", err),
            }
        }

        false
    }

    /// Attempt to redraw the surface using the current color
    fn redraw(&mut self) -> Result<(), DrawError> {
        let pool = self.pools.pool().map_or(Err(DrawError::NoFreePool), Ok)?;

        let stride = 4 * self.dimensions.0;
        let width = self.dimensions.0;
        let height = self.dimensions.1;

        // First make sure the pool is large enough
        pool.resize((stride * height) as usize)?;

        // Create a new buffer from the pool
        let buffer =
            pool.buffer(0, width as i32, height as i32, stride as i32, wl_shm::Format::Argb8888);

        let font = &self.fonts;

        let start = std::time::Instant::now();

        let ptr = pool.mmap().as_mut_ptr() as *mut u8;
        let mut canvas =
            Canvas { mem: ptr, dimensions: (width, height), color: self.color, fonts: font };

        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            max_width: Some(self.dimensions.0 as f32),
            max_height: Some(self.dimensions.0 as f32 / 2f32),
            horizontal_align: HorizontalAlign::Center,
            vertical_align: VerticalAlign::Middle,
            ..LayoutSettings::default()
        });

        let text = {
            let time = chrono::prelude::Local::now();
            format!("{:02}:{:02}\n", time.hour(), time.minute())
        };

        layout.append(font, &TextStyle::new(&text, 64.0, 0));

        let text = format!("User: {}\n", users::get_current_username().unwrap().to_str().unwrap());
        layout.append(font, &TextStyle::new(&text, 32.0, 0));
        let text = format!("pwd: {}", "*".to_string().repeat(min(self.chars_entered, 64) as usize));
        layout.append(font, &TextStyle::new(&text, 32.0, 0));

        canvas.color = 0xff000000;
        canvas.fill();
        canvas.color = 0xffffffff;
        canvas.draw_layout(&mut layout);
        canvas.color = self.color;
        canvas.draw_square((450, height / 2 + 50), (width - 450, height / 2 + 60));

        println!("{}", start.elapsed().as_secs_f64());

        // Attach the buffer to the surface and mark the entire surface as damaged
        self.surface.attach(Some(&buffer), 0, 0);
        self.surface.damage_buffer(0, 0, width as i32, height as i32);

        // Finally, commit the surface
        self.surface.commit();

        Ok(())
    }
}

impl Drop for LockSurface {
    fn drop(&mut self) {
        self.layer_surface.destroy();
        self.surface.destroy();
    }
}
