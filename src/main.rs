#![allow(dead_code)]

use clap::{
    crate_authors, crate_description, crate_name, crate_version, App, AppSettings, Arg, SubCommand,
};
use git2::Repository;
use std::convert::TryFrom;
use std::env;
use std::fmt::Write;
use std::fs;
use std::time::Duration;

const MIN_CMD_EXEC_TIME: Duration = Duration::from_secs(2);

fn elapsed_seconds_validator(s: String) -> Result<(), String> {
    if s.parse::<usize>().is_err() {
        Err("The argument must be a valid positive integer".into())
    } else {
        Ok(())
    }
}

#[inline]
fn humanize_duration(dur: &Duration) -> String {
    let secs = dur.as_secs();
    if secs == 0 {
        return Default::default();
    }
    if secs < 60 {
        return format!("{}s", secs);
    }
    if secs < 3600 {
        return format!(
            "{}m {}",
            (secs as f32 / 60.0).trunc() as usize,
            humanize_duration(&Duration::from_secs(secs % 60))
        )
        .trim_end()
        .into();
    }
    return format!(
        "{}h {}",
        (secs as f32 / 3600.0).trunc() as usize,
        humanize_duration(&Duration::from_secs(secs % 3600)),
    )
    .trim_end()
    .into();
}

#[test]
fn test_humanize_duration() {
    assert_eq!("", humanize_duration(&Duration::from_secs(0)));
    assert_eq!("59s", humanize_duration(&Duration::from_secs(59)));
    assert_eq!("1m", humanize_duration(&Duration::from_secs(60)));
    assert_eq!("1m 59s", humanize_duration(&Duration::from_secs(60 + 59)));
    assert_eq!("2m 1s", humanize_duration(&Duration::from_secs(60 * 2 + 1)));
    assert_eq!(
        "59m 59s",
        humanize_duration(&Duration::from_secs(60 * 59 + 59))
    );
    assert_eq!("1h", humanize_duration(&Duration::from_secs(60 * 60)));
    assert_eq!(
        "1h 1s",
        humanize_duration(&Duration::from_secs(60 * 60 + 1))
    );
    assert_eq!(
        "34h 59m 59s",
        humanize_duration(&Duration::from_secs(60 * 60 * 34 + 60 * 59 + 59))
    );
}

fn get_current_branch(repo: &Repository) -> Option<String> {
    let head = match repo.head() {
        Ok(reference) => reference,
        Err(e) => {
            return if e.code() == git2::ErrorCode::UnbornBranch {
                // HEAD should only be an unborn branch if the repository is fresh, in that case
                // read directly from `.git/HEAD`.
                let mut head_path = repo.path().to_path_buf();
                head_path.push("HEAD");
                // Get the first line, then last path segment.
                std::fs::read_to_string(&head_path)
                    .ok()?
                    .lines()
                    .next()?
                    .trim()
                    .split('/')
                    .last()
                    .map(|r| r.to_owned())
            } else {
                None
            };
        }
    };
    let shorthand = head.shorthand();
    shorthand.map(|x| x.into())
}

fn main() {
    let matches = App::new(crate_name!())
        .version(crate_version!())
        .author(crate_authors!())
        .about(crate_description!())
        .settings(&[AppSettings::SubcommandRequired])
        .global_settings(&[AppSettings::ColorAuto, AppSettings::ColoredHelp])
        .subcommand(
            SubCommand::with_name("prompt")
                .about("Output the prompt string")
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
                        .possible_values(&Shell::SUPPORTED),
                )
                .arg(
                    Arg::with_name("elapsed_seconds")
                        .long("elapsed-seconds")
                        .takes_value(true)
                        .help("Last command's execution time in seconds")
                        .required(true)
                        .validator(elapsed_seconds_validator),
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
                ),
        )
        .subcommand(
            SubCommand::with_name("init")
                .about("Output code to be evaluated by the shell to init the prompt")
                .arg(
                    Arg::with_name("shell")
                        .long("shell")
                        .short("s")
                        .takes_value(true)
                        .help("The shell for which to output the init code")
                        .required(true)
                        .possible_values(&Shell::SUPPORTED),
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
                ),
        )
        .get_matches();

    match matches.subcommand() {
        ("prompt", Some(matches)) => {
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
                Some(git.as_ref().and_then(|x| x.toplevel()))
            } else {
                None
            })
            .unwrap_or("??".into());

            // TODO(agnipau): Windows support.
            let is_root = unsafe { libc::getuid() } == 0;

            // parse can't fail, we checked this using clap.
            let elapsed: usize = matches
                .value_of("elapsed_seconds")
                .unwrap()
                .parse()
                .unwrap();
            let elapsed = Duration::from_secs(elapsed as u64);

            let mut s = String::new();
            if is_root {
                let _ = write!(
                    &mut s,
                    "{}{}root{} in ",
                    Attribute::Bold.to_str(shell),
                    Color::Red.to_str(false, shell),
                    Attribute::Reset.to_str(shell)
                );
            }
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
            if elapsed >= MIN_CMD_EXEC_TIME {
                let _ = write!(
                    &mut s,
                    "{}took {} ",
                    Color::Yellow.to_str(false, shell),
                    humanize_duration(&elapsed),
                );
            }
            let _ = write!(
                &mut s,
                "{}{}{}",
                if non_zero_exit_status {
                    Color::Red.to_str(false, shell)
                } else {
                    Color::Green.to_str(false, shell)
                },
                separator_symbol,
                Attribute::Reset.to_str(shell),
            );
            if *shell != Shell::Bash {
                let _ = write!(&mut s, " ");
            }

            print!("{}", s);
        }
        ("init", Some(matches)) => {
            let mut args = String::from(" ");
            let unicode = matches.is_present("unicode");
            if unicode {
                args.push_str("-u ");
            }
            let short_path = matches.is_present("short_path");
            if short_path {
                args.push_str("-p ");
            }
            let args = args.trim_end();

            let shell = Shell::try_from(matches.value_of("shell").unwrap()).unwrap();
            println!("{}", shell.init_code(args));
        }
        _ => unreachable!(),
    }
}

struct Git {
    repo: Repository,
}

impl Git {
    fn new() -> Option<Self> {
        let repo = Repository::discover(env::current_dir().ok()?).ok()?;
        Some(Self { repo })
    }

    // https://github.com/starship/starship/blob/d670212a083e9f1e9c9a2313e7ce5e72e908efa7/src/context.rs
    fn branch(&self) -> Option<String> {
        let head = match self.repo.head() {
            Ok(reference) => reference,
            Err(e) => {
                return if e.code() == git2::ErrorCode::UnbornBranch {
                    // HEAD should only be an unborn branch if the repository is fresh, in that case
                    // read directly from `.git/HEAD`.
                    let mut head_path = self.repo.path().to_path_buf();
                    head_path.push("HEAD");
                    // Get the first line, then last path segment.
                    fs::read_to_string(&head_path)
                        .ok()?
                        .lines()
                        .next()?
                        .trim()
                        .split('/')
                        .last()
                        .map(|r| r.to_owned())
                } else {
                    None
                };
            }
        };
        let shorthand = head.shorthand();
        shorthand.map(|x| x.into())
    }

    fn toplevel(&self) -> Option<&str> {
        Some(self.repo.workdir()?.file_name()?.to_str()?)
    }
}

/// If `short` is None, the full path will be returned.
/// If `short` is Some, a shorter variant will be returned, in this case we also need to know the
/// repo name.
type InsideGitRepo<'a> = Option<&'a str>;
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

    match short {
        // Short path inside git tree.
        Some(Some(toplevel)) => {
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
        // Short path NOT inside git tree.
        Some(None) => {
            let mut parts = path.split('/').rev().take(3).collect::<Vec<_>>();
            parts.reverse();
            Some(parts.join("/"))
        }
        // Full path.
        None => Some(path),
    }
}

#[derive(PartialEq)]
enum Shell {
    Zsh,
    Bash,
}

impl Shell {
    const SUPPORTED: [&'static str; 2] = ["zsh", "bash"];

    fn init_code(&self, args: &str) -> String {
        match self {
            Self::Zsh => format!(
                r#"
preexec() {{
    _sprompt_preexec_ran=true
    _sprompt_last_seconds="$SECONDS"
}}
precmd() {{
    if [ "$_sprompt_preexec_ran" = false ]; then
        _sprompt_last_seconds="$SECONDS"
    fi
    _sprompt_preexec_ran=false
}}
setopt PROMPT_SUBST
PROMPT="\$(sprompt prompt -e "\$?" -s zsh --elapsed-seconds "\$(( SECONDS - _sprompt_last_seconds ))"{args})"
"#,
                args = args
            )
            .trim()
            .into(),
            Self::Bash => format!(
                r#"
_sprompt_beforecmd() {{
    [ "${{_sprompt_beforecmd_ran:=false}}" = true ] && return
    _sprompt_beforecmd_ran=true
    _sprompt_last_seconds="$SECONDS"
}}
trap _sprompt_beforecmd DEBUG

_sprompt_aftercmd() {{
    STATUS="$?"
    if [ "${{_sprompt_aftercmd_first_run:=true}}" = true ]; then
        _sprompt_aftercmd_first_run=false
    else
        _sprompt_beforecmd_ran=false
        if [ "$_sprompt_last_seconds" -eq 0 ]; then
            _sprompt_last_seconds="$SECONDS"
        fi
        _sprompt_elapsed_seconds="$(( SECONDS - _sprompt_last_seconds ))"
    fi
    sprompt prompt -e "$STATUS" -s bash --elapsed-seconds "${{_sprompt_elapsed_seconds:=0}}"{args}
}}
# If PS1 is completely empty, pressing the <enter> key doesn't work.
PS1=\ 
PROMPT_COMMAND=_sprompt_aftercmd
"#,
                args = args
            )
            .trim()
            .into(),
        }
    }
}

impl TryFrom<&str> for Shell {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            "zsh" => Ok(Self::Zsh),
            "bash" => Ok(Self::Bash),
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
    // TODO(agnipau): Windows support.
    const fn to_str(&self, bright: bool, shell: &Shell) -> &str {
        match self {
            Self::Black => match shell {
                Shell::Bash => {
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
                Shell::Bash => {
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
                Shell::Bash => {
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
                Shell::Bash => {
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
                Shell::Bash => {
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
                Shell::Bash => {
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
                Shell::Bash => {
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
                Shell::Bash => {
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
    // TODO(agnipau): Windows support.
    const fn to_str(&self, shell: &Shell) -> &str {
        match self {
            Self::Reset => match shell {
                Shell::Bash => "\u{001b}[0m",
                Shell::Zsh => "%{\u{001b}[0m%}",
            },
            Self::Bold => match shell {
                Shell::Bash => "\u{001b}[1m",
                Shell::Zsh => "%{\u{001b}[1m%}",
            },
            Self::Underline => match shell {
                Shell::Bash => "\u{001b}[4m",
                Shell::Zsh => "%{\u{001b}[4m%}",
            },
            Self::Reversed => match shell {
                Shell::Bash => "\u{001b}[7m",
                Shell::Zsh => "%{\u{001b}[7m%}",
            },
        }
    }
}
