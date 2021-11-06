use std::io;
use std::process::exit;

use log::error;
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{fork, ForkResult};

use config::Config;

use crate::lock::lock_screen;
use std::sync::Arc;

mod config;
mod lock;
mod logger;

fn main() -> io::Result<()> {
    let options = match Config::new() {
        Ok(config) => Arc::new(config),
        Err(err) => {
            println!("{}", err);
            error!("{:?}", err);
            exit(1);
        }
    };
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
                Ok(WaitStatus::Exited(_pid, 0)) => exit(0),
                a => error!("[MAIN] waitpid() didn't behave as expected. Code: {:?}", a),
            },
            Err(errno) => {
                error!("[MAIN] couldn't fork(). ERRNO: {}", errno);
                exit(1);
            }
        }
    }
}
