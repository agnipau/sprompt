#![allow(dead_code)]

use clap::{crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg};
use std::convert::TryFrom;
use std::env;
use std::fmt::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn main() {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .settings(&[AppSettings::ColorAuto, AppSettings::ColoredHelp])
        .arg(
            Arg::with_name("exit_code")
                .short("e")
                .takes_value(true)
                .help("Last command exit code")
                .required(true),
        )
        .arg(
            Arg::with_name("shell")
                .short("s")
                .takes_value(true)
                .help("The shell where the prompt will be shown")
                .required(true)
                .possible_values(&["zsh", "posix"]),
        )
        .get_matches();

    let non_zero_exit_status = matches.value_of("exit_code").unwrap() != "0";
    let ref shell = Shell::try_from(matches.value_of("shell").unwrap()).unwrap();

    let git = Git::default();
    let cwd = get_cwd().unwrap_or("??".into());

    let mut s = String::new();
    let _ = write!(
        &mut s,
        "{}{}{} ",
        Attribute::Bold.to_str(shell),
        Color::Cyan.to_str(false, shell),
        cwd
    );
    if let Some(branch) = git.branch() {
        let _ = write!(
            &mut s,
            "{}on {}{}{} ",
            Attribute::Reset.to_str(shell),
            Attribute::Bold.to_str(shell),
            Color::Magenta.to_str(false, shell),
            branch
        );
        // TODO(agnipau): Checking for git dirty state in a decently performant way in big repos
        // (like UnrealEngine) is quite difficult.
    }
    let _ = write!(
        &mut s,
        "{}::{} ",
        if non_zero_exit_status {
            Color::Red.to_str(false, shell)
        } else {
            Color::Green.to_str(false, shell)
        },
        Attribute::Reset.to_str(shell),
    );

    print!("{}", s);
}

struct Git {
    inside_repo: bool,
}

impl Default for Git {
    fn default() -> Self {
        Self {
            inside_repo: Path::new(".git").exists(),
        }
    }
}

impl Git {
    // https://stackoverflow.com/a/2659808
    fn has_staged_changes(&self) -> Option<bool> {
        if self.inside_repo {
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
        } else {
            None
        }
    }

    // https://stackoverflow.com/a/2659808
    fn has_changes_to_stage(&self) -> Option<bool> {
        if self.inside_repo {
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
        } else {
            None
        }
    }

    // https://stackoverflow.com/a/2659808
    fn has_staged_and_changes_to_stage(&self) -> Option<bool> {
        if self.inside_repo {
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
        } else {
            None
        }
    }

    // https://stackoverflow.com/a/2659808
    fn has_untracked_files(&self) -> Option<bool> {
        if self.inside_repo {
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
        } else {
            None
        }
    }

    // https://stackoverflow.com/a/11868440
    fn branch(&self) -> Option<String> {
        if self.inside_repo {
            let git_branch = String::from_utf8(
                Command::new("git")
                    .args(&["symbolic-ref", "--short", "HEAD"])
                    .output()
                    .ok()?
                    .stdout,
            )
            .ok()?;
            Some(git_branch.trim_end().to_owned())
        } else {
            None
        }
    }
}

fn get_cwd() -> Option<String> {
    let mut cwd = env::current_dir().ok()?.to_str()?.to_owned();
    Some(if cwd.starts_with("/home/") {
        cwd = cwd.replace("/home/", "");
        if let Some(idx) = cwd.find("/") {
            cwd.replace_range(..idx, "");
            cwd.insert(0, '~');
            cwd
        } else {
            "~".to_owned()
        }
    } else {
        cwd
    })
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
