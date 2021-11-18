use std::collections::VecDeque;
use std::io;
use std::process::exit;

use log::error;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{fork, ForkResult};

use config::Config;

use crate::lock::lock_screen;
use std::sync::Arc;
use std::time::Instant;

mod config;
mod lock;
mod logger;

fn main() -> io::Result<()> {
    let options = match Config::new() {
        Ok(config) => Arc::new(config),
        Err(err) => {
            eprintln!("{}", err);
            error!("{:?}", err);
            exit(1);
        }
    };

    let mut restarts: VecDeque<Instant> = VecDeque::with_capacity(options.max_restarts);

    loop {
        match unsafe { fork() } {
            Ok(ForkResult::Child) => match lock_screen(options) {
                Ok(()) => exit(0),
                Err(err) => {
                    error!("[MAIN] lock_screen error: {:?}", err);
                    exit(1);
                }
            },
            Ok(ForkResult::Parent { child }) => match waitpid(child, None) {
                Ok(WaitStatus::Exited(_pid, code)) if code <= 1 => exit(0),
                a => {
                    error!("[MAIN] waitpid() didn't behave as expected. Code: {:?}", a);
                    if restarts.len() == options.max_restarts {
                        match restarts.pop_front() {
                            Some(ts) if ts.elapsed().as_secs() < 1 => exit(1),
                            _ => {}
                        }
                    }
                    restarts.push_back(Instant::now())
                },
            },
            Err(errno) => {
                error!("[MAIN] couldn't fork(). ERRNO: {}", errno);
                exit(1);
            }
        }
    }
}
