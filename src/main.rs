#![allow(dead_code)]

use clap::{crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg};
use std::convert::TryFrom;
use std::env;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .settings(&[AppSettings::ColorAuto, AppSettings::ColoredHelp])
        .arg(
            Arg::with_name("exit_code")
                .long("exit-code")
                .short("e")
                .takes_value(true)
                .help("Last command exit code")
                .required(true),
        )
        .arg(
            Arg::with_name("shell")
                .long("shell")
                .short("s")
                .takes_value(true)
                .help("The shell where the prompt will be shown")
                .required(true)
                .possible_values(&["zsh", "posix"]),
        )
        .arg(
            Arg::with_name("unicode")
                .long("unicode")
                .short("u")
                .help("Use unicode symbols"),
        )
        .arg(
            Arg::with_name("short_path")
                .long("short-path")
                .short("p")
                .help("Show the current path in a reduced form"),
        )
        .get_matches();

    let non_zero_exit_status = matches.value_of("exit_code").unwrap() != "0";
    let ref shell = Shell::try_from(matches.value_of("shell").unwrap()).unwrap();

    let use_unicode = matches.is_present("unicode");
    let branch_symbol = if use_unicode {
        " "
    } else {
        Default::default()
    };
    let separator_symbol = if use_unicode { "❯" } else { "::" };

    let git = Git::new();

    let use_short_path = matches.is_present("short_path");
    let path = get_current_path(if use_short_path {
        Some(git.as_ref().map(|x| x.toplevel.as_ref()))
    } else {
        None
    })
    .unwrap_or("??".into());

    let mut s = String::new();
    let _ = write!(
        &mut s,
        "{}{}{} ",
        Attribute::Bold.to_str(shell),
        Color::Cyan.to_str(false, shell),
        path
    );
    if let Some(branch) = git.and_then(|x| x.branch()) {
        let _ = write!(
            &mut s,
            "{}on {}{}{}{} ",
            Attribute::Reset.to_str(shell),
            Attribute::Bold.to_str(shell),
            Color::Magenta.to_str(false, shell),
            branch_symbol,
            branch
        );
        // TODO(agnipau): Checking for git dirty state in a decently performant way in big repos
        // (like UnrealEngine) is quite difficult.
    }
    let _ = write!(
        &mut s,
        "{}{}{} ",
        if non_zero_exit_status {
            Color::Red.to_str(false, shell)
        } else {
            Color::Green.to_str(false, shell)
        },
        separator_symbol,
        Attribute::Reset.to_str(shell),
    );

    print!("{}", s);
}

struct Git {
    toplevel: PathBuf,
}

impl Git {
    fn new() -> Option<Self> {
        Some(Self {
            toplevel: Git::toplevel()?,
        })
    }

    // https://stackoverflow.com/a/16925062
    fn toplevel() -> Option<PathBuf> {
        let stdout = Command::new("git")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .args(&["rev-parse", "--show-toplevel"])
            .output()
            .ok()?
            .stdout;
        if stdout.is_empty() {
            None
        } else {
            Some(PathBuf::from(String::from_utf8(stdout).ok()?.trim_end()))
        }
    }

    // https://stackoverflow.com/a/2659808
    fn has_staged_changes(&self) -> Option<bool> {
        Some(
            Command::new("git")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .args(&["diff-index", "--quiet", "--cached", "HEAD", "--"])
                .status()
                .ok()?
                .code()?
                == 1,
        )
    }

    // https://stackoverflow.com/a/2659808
    fn has_changes_to_stage(&self) -> Option<bool> {
        Some(
            Command::new("git")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .args(&["diff-files", "--quiet"])
                .status()
                .ok()?
                .code()?
                == 1,
        )
    }

    // https://stackoverflow.com/a/2659808
    fn has_staged_and_changes_to_stage(&self) -> Option<bool> {
        Some(
            Command::new("git")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .args(&["diff-index", "--quiet", "HEAD", "--"])
                .status()
                .ok()?
                .code()?
                == 1,
        )
    }

    // https://stackoverflow.com/a/2659808
    fn has_untracked_files(&self) -> Option<bool> {
        Some(
            !Command::new("git")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .args(&["ls-files", "--others"])
                .output()
                .ok()?
                .stdout
                .is_empty(),
        )
    }

    // https://stackoverflow.com/a/11868440
    fn branch(&self) -> Option<String> {
        let git_branch = String::from_utf8(
            Command::new("git")
                .args(&["symbolic-ref", "--short", "HEAD"])
                .output()
                .ok()?
                .stdout,
        )
        .ok()?;
        Some(git_branch.trim_end().to_owned())
    }
}

/// If `short` is None, the full path will be returned.
/// If `short` is Some, a shorter variant will be returned, in this case we also need to know the
/// repo name.
type InsideGitRepo<'a> = Option<&'a Path>;
type Short<'a> = Option<InsideGitRepo<'a>>;
fn get_current_path(short: Short) -> Option<String> {
    let path = env::current_dir().ok()?;

    let path = path.to_str()?.to_owned();
    let path = if path.starts_with("/home/") {
        let mut path = path.replace("/home/", "");
        if let Some(idx) = path.find("/") {
            path.replace_range(..idx, "");
            path.insert(0, '~');
            path
        } else {
            "~".to_owned()
        }
    } else {
        path
    };

    let short = short.map(|x| x.map(|y| y.file_name().and_then(|z| z.to_str())));
    match short {
        // Short path inside git tree and toplevel is valid.
        Some(Some(Some(toplevel))) => {
            let mut parts = path.split('/').rev().take(3).collect::<Vec<_>>();
            parts.reverse();
            if parts[1] == toplevel {
                parts.remove(0);
            } else if parts[2] == toplevel {
                parts.remove(0);
                parts.remove(0);
            }
            Some(parts.join("/"))
        }
        // Short path NOT inside git tree or toplevel is not valid.
        Some(None) | Some(Some(None)) => {
            let mut parts = path.split('/').rev().take(3).collect::<Vec<_>>();
            parts.reverse();
            Some(parts.join("/"))
        }
        // Full path.
        None => Some(path),
    }
}

enum Shell {
    Zsh,
    Posix,
}

impl TryFrom<&str> for Shell {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "zsh" => Ok(Self::Zsh),
            "posix" => Ok(Self::Posix),
            _ => Err(()),
        }
    }
}

enum Color {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Color {
    fn to_str(&self, bright: bool, shell: &Shell) -> &str {
        match self {
            Self::Black => match shell {
                Shell::Posix => {
                    if bright {
                        "\u{001b}[30;1m"
                    } else {
                        "\u{001b}[30m"
                    }
                }
                Shell::Zsh => {
                    if bright {
                        "%{\u{001b}[30;1m%}"
                    } else {
                        "%{\u{001b}[30m%}"
                    }
                }
            },
            Self::Red => match shell {
                Shell::Posix => {
                    if bright {
                        "\u{001b}[31;1m"
                    } else {
                        "\u{001b}[31m"
                    }
                }
                Shell::Zsh => {
                    if bright {
                        "%{\u{001b}[31;1m%}"
                    } else {
                        "%{\u{001b}[31m%}"
                    }
                }
            },
            Self::Green => match shell {
                Shell::Posix => {
                    if bright {
                        "\u{001b}[32;1m"
                    } else {
                        "\u{001b}[32m"
                    }
                }
                Shell::Zsh => {
                    if bright {
                        "%{\u{001b}[32;1m%}"
                    } else {
                        "%{\u{001b}[32m%}"
                    }
                }
            },
            Self::Yellow => match shell {
                Shell::Posix => {
                    if bright {
                        "\u{001b}[33;1m"
                    } else {
                        "\u{001b}[33m"
                    }
                }
                Shell::Zsh => {
                    if bright {
                        "%{\u{001b}[33;1m%}"
                    } else {
                        "%{\u{001b}[33m%}"
                    }
                }
            },
            Self::Blue => match shell {
                Shell::Posix => {
                    if bright {
                        "\u{001b}[34;1m"
                    } else {
                        "\u{001b}[34m"
                    }
                }
                Shell::Zsh => {
                    if bright {
                        "%{\u{001b}[34;1m%}"
                    } else {
                        "%{\u{001b}[34m%}"
                    }
                }
            },
            Self::Magenta => match shell {
                Shell::Posix => {
                    if bright {
                        "\u{001b}[35;1m"
                    } else {
                        "\u{001b}[35m"
                    }
                }
                Shell::Zsh => {
                    if bright {
                        "%{\u{001b}[35;1m%}"
                    } else {
                        "%{\u{001b}[35m%}"
                    }
                }
            },
            Self::Cyan => match shell {
                Shell::Posix => {
                    if bright {
                        "\u{001b}[36;1m"
                    } else {
                        "\u{001b}[36m"
                    }
                }
                Shell::Zsh => {
                    if bright {
                        "%{\u{001b}[36;1m%}"
                    } else {
                        "%{\u{001b}[36m%}"
                    }
                }
            },
            Self::White => match shell {
                Shell::Posix => {
                    if bright {
                        "\u{001b}[37;1m"
                    } else {
                        "\u{001b}[37m"
                    }
                }
                Shell::Zsh => {
                    if bright {
                        "%{\u{001b}[37;1m%}"
                    } else {
                        "%{\u{001b}[37m%}"
                    }
                }
            },
        }
    }
}

enum Attribute {
    Reset,
    Bold,
    Underline,
    Reversed,
}

impl Attribute {
    fn to_str(&self, shell: &Shell) -> &str {
        match self {
            Self::Reset => match shell {
                Shell::Posix => "\u{001b}[0m",
                Shell::Zsh => "%{\u{001b}[0m%}",
            },
            Self::Bold => match shell {
                Shell::Posix => "\u{001b}[1m",
                Shell::Zsh => "%{\u{001b}[1m%}",
            },
            Self::Underline => match shell {
                Shell::Posix => "\u{001b}[4m",
                Shell::Zsh => "%{\u{001b}[4m%}",
            },
            Self::Reversed => match shell {
                Shell::Posix => "\u{001b}[7m",
                Shell::Zsh => "%{\u{001b}[7m%}",
            },
        }
    }
}
