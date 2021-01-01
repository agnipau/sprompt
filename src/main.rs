#![allow(dead_code)]

use std::env;
use std::fmt::Write;
use std::path::Path;
use std::process::{Command, Stdio};

fn main() {
    let non_zero_exit_status = env::args()
        .skip(1)
        .next()
        .map(|x| &x != "0")
        .unwrap_or(false);
    let git = Git::default();
    let cwd = get_cwd().unwrap_or("??".into());

    let mut s = String::new();
    let _ = write!(
        &mut s,
        "{}{}{} ",
        Attribute::Bold.to_str(&TargetShell::Zsh),
        Color::Cyan.to_str(false, &TargetShell::Zsh),
        cwd
    );
    if let Some(branch) = git.branch() {
        let _ = write!(
            &mut s,
            "{}on {}{}{} ",
            Attribute::Reset.to_str(&TargetShell::Zsh),
            Attribute::Bold.to_str(&TargetShell::Zsh),
            Color::Magenta.to_str(false, &TargetShell::Zsh),
            branch
        );
        // TODO(agnipau): Checking for git dirty state in a decently performant way in big repos
        // (like UnrealEngine) is quite difficult.
    }
    let _ = write!(
        &mut s,
        "{}::{} ",
        if non_zero_exit_status {
            Color::Red.to_str(false, &TargetShell::Zsh)
        } else {
            Color::Green.to_str(false, &TargetShell::Zsh)
        },
        Attribute::Reset.to_str(&TargetShell::Zsh),
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

enum TargetShell {
    Zsh,
    Posix,
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
    fn to_str(&self, bright: bool, target_shell: &TargetShell) -> &str {
        match self {
            Self::Black => match target_shell {
                TargetShell::Posix => {
                    if bright {
                        "\u{001b}[30;1m"
                    } else {
                        "\u{001b}[30m"
                    }
                }
                TargetShell::Zsh => {
                    if bright {
                        "%{\u{001b}[30;1m%}"
                    } else {
                        "%{\u{001b}[30m%}"
                    }
                }
            },
            Self::Red => match target_shell {
                TargetShell::Posix => {
                    if bright {
                        "\u{001b}[31;1m"
                    } else {
                        "\u{001b}[31m"
                    }
                }
                TargetShell::Zsh => {
                    if bright {
                        "%{\u{001b}[31;1m%}"
                    } else {
                        "%{\u{001b}[31m%}"
                    }
                }
            },
            Self::Green => match target_shell {
                TargetShell::Posix => {
                    if bright {
                        "\u{001b}[32;1m"
                    } else {
                        "\u{001b}[32m"
                    }
                }
                TargetShell::Zsh => {
                    if bright {
                        "%{\u{001b}[32;1m%}"
                    } else {
                        "%{\u{001b}[32m%}"
                    }
                }
            },
            Self::Yellow => match target_shell {
                TargetShell::Posix => {
                    if bright {
                        "\u{001b}[33;1m"
                    } else {
                        "\u{001b}[33m"
                    }
                }
                TargetShell::Zsh => {
                    if bright {
                        "%{\u{001b}[33;1m%}"
                    } else {
                        "%{\u{001b}[33m%}"
                    }
                }
            },
            Self::Blue => match target_shell {
                TargetShell::Posix => {
                    if bright {
                        "\u{001b}[34;1m"
                    } else {
                        "\u{001b}[34m"
                    }
                }
                TargetShell::Zsh => {
                    if bright {
                        "%{\u{001b}[34;1m%}"
                    } else {
                        "%{\u{001b}[34m%}"
                    }
                }
            },
            Self::Magenta => match target_shell {
                TargetShell::Posix => {
                    if bright {
                        "\u{001b}[35;1m"
                    } else {
                        "\u{001b}[35m"
                    }
                }
                TargetShell::Zsh => {
                    if bright {
                        "%{\u{001b}[35;1m%}"
                    } else {
                        "%{\u{001b}[35m%}"
                    }
                }
            },
            Self::Cyan => match target_shell {
                TargetShell::Posix => {
                    if bright {
                        "\u{001b}[36;1m"
                    } else {
                        "\u{001b}[36m"
                    }
                }
                TargetShell::Zsh => {
                    if bright {
                        "%{\u{001b}[36;1m%}"
                    } else {
                        "%{\u{001b}[36m%}"
                    }
                }
            },
            Self::White => match target_shell {
                TargetShell::Posix => {
                    if bright {
                        "\u{001b}[37;1m"
                    } else {
                        "\u{001b}[37m"
                    }
                }
                TargetShell::Zsh => {
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
    fn to_str(&self, target_shell: &TargetShell) -> &str {
        match self {
            Self::Reset => match target_shell {
                TargetShell::Posix => "\u{001b}[0m",
                TargetShell::Zsh => "%{\u{001b}[0m%}",
            },
            Self::Bold => match target_shell {
                TargetShell::Posix => "\u{001b}[1m",
                TargetShell::Zsh => "%{\u{001b}[1m%}",
            },
            Self::Underline => match target_shell {
                TargetShell::Posix => "\u{001b}[4m",
                TargetShell::Zsh => "%{\u{001b}[4m%}",
            },
            Self::Reversed => match target_shell {
                TargetShell::Posix => "\u{001b}[7m",
                TargetShell::Zsh => "%{\u{001b}[7m%}",
            },
        }
    }
}
