# waylock

---
### This is a modified fork from the 
---

Waylock is a simple screenlocker for wayland compositors. It takes inspiration
from [slock](https://tools.suckless.org/slock/) with its minimalistic feature
set, but is implemented in [rust](https://www.rust-lang.org/) for first class
safety and security.

Waylock will work with any wayland compositor implementing the `wlr-layer-shell` and
`wlr-input-inhibitor` protocols. In general, this means
[wlroots](https://github.com/swaywm/wlroots)-based compositors such as
[river](https://github.com/ifreund/river) or
[sway](https://github.com/swaywm/sway).

### Installation

Waylock can be manually compiled from source or installed using
[cargo](https://github.com/rust-lang/cargo). Note that waylock links against
`libpam` and you will need the relevant headers installed on your system to
build waylock.

```sh
$ cargo install waylock --locked
```

It is also packaged for several linux distributions:
[https://repology.org/project/waylock/versions](https://repology.org/project/waylock/versions).

### Usage

```
USAGE:
    waylock [FLAGS] [OPTIONS]

FLAGS:
    -h, --help       Prints help information
        --one-way    Never revert the color after input or failure.
    -v               Enable verbose logging, repeat for greater effect (e.g. -vvv).
    -V, --version    Prints version information

OPTIONS:
        --config <FILE>
            Use an alternative config file. [default: $XDG_CONFIG_HOME/waylock/waylock.toml]

        --fail-color <COLOR>
            Set the color of the lock screen on authentication failure. [default: #ff0000]

        --fail-command <COMMAND>
            Command to run on authentication failure. Executed with `sh -c <COMMAND>`.

        --init-color <COLOR>
            Set the initial color of the lock screen. [default: #ffffff]

        --input-color <COLOR>
            Set the color of the lock screen after input is received. [default: #0000ff]
```

Detaching waylock from the controlling terminal to run as a daemon can be accomplished with `setsid(1)`.

Some examples of what `--fail-command` could be used for include:
- Playing an alarm sound
- Taking a screenshot with the webcam
- Sending an email to yourself
