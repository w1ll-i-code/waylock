use std::borrow::BorrowMut;
use std::io;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};

use smithay_client_toolkit::{
    reexports::{
        calloop,
        client::protocol::{wl_compositor, wl_shm},
        protocols::wlr::unstable::input_inhibitor::v1::client::zwlr_input_inhibit_manager_v1,
        protocols::wlr::unstable::layer_shell::v1::client::zwlr_layer_shell_v1,
    },
    seat::keyboard::keysyms,
    WaylandSource,
};

use self::auth::LockAuth;
use self::env::LockEnv;
use self::input::LockInput;
use self::output::OutputHandling;
use self::surface::LockSurface;
use crate::config::Config;

mod auth;
mod env;
mod input;
mod output;
mod surface;
mod canvas;

#[derive(Copy, Clone, PartialEq)]
pub enum LockState {
    Init,
    Input,
    Fail,
}

impl LockState {
    fn map_to_color(&self, config: Arc<Config>) -> u32 {
        match self {
            LockState::Init => config.colors.init_color,
            LockState::Input => config.colors.input_color,
            LockState::Fail => config.colors.fail_color,
        }
    }
}

pub fn lock_screen(config: Arc<Config>) -> io::Result<()> {
    let (lock_env, display, queue) = LockEnv::init_environment()?;

    let _inhibitor = lock_env
        .require_global::<zwlr_input_inhibit_manager_v1::ZwlrInputInhibitManagerV1>()
        .get_inhibitor();

    let lock_surfaces = {
        let compositor = lock_env.require_global::<wl_compositor::WlCompositor>();
        let layer_shell = lock_env.require_global::<zwlr_layer_shell_v1::ZwlrLayerShellV1>();
        let shm = lock_env.require_global::<wl_shm::WlShm>();

        let lock_surfaces = Arc::new(Mutex::new(Vec::new()));

        let lock_surfaces_handle = lock_surfaces.clone();
        let config2 = config.clone();
        lock_env.set_output_created_listener(Some(move |id, output| {
            lock_surfaces_handle.lock().unwrap().borrow_mut().push((
                id,
                LockSurface::new(
                    &output,
                    &compositor.clone(),
                    &layer_shell.clone(),
                    shm.clone(),
                    config2.clone(),
                ),
            ));
        }));

        let lock_surfaces_handle = lock_surfaces.clone();
        lock_env.set_output_removed_listener(Some(move |id| {
            lock_surfaces_handle.lock().unwrap().borrow_mut().retain(|(i, _)| *i != id);
        }));

        lock_surfaces
    };

    let mut event_loop = calloop::EventLoop::new()?;

    let lock_input = LockInput::new(&lock_env, event_loop.handle());

    WaylandSource::new(queue).quick_insert(event_loop.handle())?;

    let lock_auth = LockAuth::new();
    let mut current_password = String::new();

    let mut lock_state = LockState::Init;

    let set_state = |state, num| {
        for (_, lock_surface) in lock_surfaces.lock().unwrap().borrow_mut().iter_mut() {
            lock_surface.set_state(state);
            lock_surface.chars_entered(num);
            lock_surface.set_redraw();
        }
    };

    let timer = calloop::timer::Timer::new().unwrap();
    let timer_handle = timer.handle();
    timer_handle
        .add_timeout(Duration::from_secs(60 - UNIX_EPOCH.elapsed().unwrap().as_secs() % 60), ());

    let surface_ref = lock_surfaces.clone();
    event_loop.handle().insert_source(timer, move |_event, metadata, _shared_data| {
        for (_, lock_surface) in surface_ref.lock().unwrap().borrow_mut().iter_mut() {
            lock_surface.set_redraw();
            lock_surface.handle_events();
        }
        metadata.cancel_all_timeouts();
        metadata.add_timeout(Duration::from_secs(60), ());
    })?;

    loop {
        // Handle all input received since last check
        while let Some((keysym, utf8)) = lock_input.pop() {
            match keysym {
                keysyms::XKB_KEY_KP_Enter | keysyms::XKB_KEY_Return => {
                    match lock_auth.check_password(&current_password) {
                        Ok(()) => return Ok(()),
                        Err(err) => {
                            log::warn!("Authentication failure {}", err);
                            lock_state = LockState::Fail;
                            set_state(lock_state, 0);
                            current_password = String::new();

                            if let Some(command) = &config.fail_command {
                                if let Err(err) = Command::new("sh").arg("-c").arg(command.to_owned()).spawn() {
                                    log::warn!("Error executing fail command \"{}\": {}", command, err);
                                }
                            }

                            continue;
                        }
                    }
                }
                keysyms::XKB_KEY_Delete | keysyms::XKB_KEY_BackSpace => {
                    current_password.pop();
                }
                keysyms::XKB_KEY_Escape => {
                    current_password.clear();
                }
                _ => {
                    if let Some(new_input) = utf8 {
                        current_password.push_str(&new_input);
                    }
                }
            }
            if current_password.is_empty() {
                if lock_state != LockState::Fail {
                    lock_state = LockState::Init;
                    set_state(lock_state, 0);
                }
            } else {
                lock_state = LockState::Input;
                set_state(lock_state, current_password.len() as u32);
            }
        }

        // This is ugly, let's hope that some version of drain_filter() gets stabilized soon
        // https://github.com/rust-lang/rust/issues/43244
        {
            let mut lock_surfaces = lock_surfaces.lock().unwrap();
            let mut i = 0;
            while i != lock_surfaces.len() {
                if lock_surfaces[i].1.handle_events() {
                    lock_surfaces.remove(i);
                } else {
                    i += 1;
                }
            }
        }

        retry_on_interrupt(|| display.flush())?;
        retry_on_interrupt(|| event_loop.dispatch(None, &mut ()))?;
    }
}

fn retry_on_interrupt<T, F: FnMut() -> io::Result<T>>(mut f: F) -> io::Result<T> {
    loop {
        match f() {
            Ok(val) => return Ok(val),
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        }
    }
}
