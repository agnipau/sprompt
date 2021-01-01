#![allow(dead_code)]

use std::env;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

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

fn main() {
    let non_zero_exit_status = env::args()
        .skip(1)
        .next()
        .map(|x| &x != "0")
        .unwrap_or(false);
    let git = Git::default();
    let cwd = get_cwd().unwrap_or("??".into());
    let bufwrt = BufferWriter::stdout(ColorChoice::Auto);

    let mut buffer = bufwrt.buffer();
    let mut color_spec = ColorSpec::new();
    let _ = buffer.set_color(color_spec.set_bold(true).set_fg(Some(Color::Cyan)));
    let _ = write!(&mut buffer, "{} ", cwd);
    if let Some(branch) = git.branch() {
        let _ = buffer.set_color(color_spec.set_bold(false).set_fg(None));
        let _ = write!(&mut buffer, "on ");
        let _ = buffer.set_color(color_spec.set_bold(true).set_fg(Some(Color::Magenta)));
        let _ = write!(&mut buffer, " {} ", branch);
        // TODO(agnipau): Checking for git dirty state in a decently performant way in big repos
        // (like UnrealEngine) is quite difficult.
        if false {
            if git.has_changes_to_stage().unwrap_or(false) {
                let _ = buffer.set_color(color_spec.set_fg(Some(Color::Red)));
                let _ = write!(&mut buffer, "[!] ");
            } else if git.has_untracked_files().unwrap_or(false) {
                let _ = buffer.set_color(color_spec.set_fg(Some(Color::Red)));
                let _ = write!(&mut buffer, "[?] ");
            }
        }
    }
    let _ = buffer.set_color(
        color_spec
            .set_bold(true)
            .set_fg(Some(if non_zero_exit_status {
                Color::Red
            } else {
                Color::Green
            })),
    );
    let _ = write!(&mut buffer, "❯ ");

    let _ = bufwrt.print(&buffer);
}
