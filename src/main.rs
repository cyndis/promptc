#![feature(path_relative_from, path_components_peek, path_ext)]

const ABBREVIATE_PARENT_DIRECTORIES: bool = true;

extern crate ansi_term;
extern crate users;

use ansi_term::Colour::{Red, Green, Fixed};
use ansi_term::Style;

use std::io::prelude::*;
use std::fs::File;
use std::{env, fs, process};
use std::path::Path;

use std::fmt::Write as FmtWrite;

type Result<T> = ::std::result::Result<T, Box<std::error::Error>>;

fn get_hostname() -> Result<String> {
    let mut f = try!(File::open("/etc/hostname"));

    let mut data = String::new();
    while try!(f.read_to_string(&mut data)) > 0 {
    }

    Ok(data.trim().into())
}

fn get_username() -> Result<String> {
    Ok(try!(env::var("USER")))
}

fn format_path(styled: bool) -> Result<String> {
    let pwd = try!(env::current_dir());
    let home = env::home_dir();

    let mut root;
    let tail;

    /* Still some allocations here.. doesn't really matter */
    if let Some(home) = home {
        if let Some(rel) = pwd.relative_from(&home) {
            // Current dir is under home
            root = "~";
            tail = rel.to_owned();
        } else {
            root = "";
            tail = pwd.relative_from("/").unwrap().to_owned();
        }
    } else {
        root = "";
        tail = pwd.relative_from("/").unwrap().to_owned();
    }

    let mut components = tail.components();

    let mut result = String::new();

    if components.peek().is_none() && root.is_empty() {
        root = "/";
    }

    let root_styled = if components.peek().is_some() {
        Fixed(7).dimmed().paint(root)
    } else {
        Style::default().bold().paint(root)
    };
    if styled {
        write!(result, "{}", root_styled).unwrap();
    } else {
        write!(result, "{}", root).unwrap();
    }

    while let Some(component) = components.next() {
        use std::path::Component::*;

        if styled {
            write!(result, "{}", Fixed(7).dimmed().paint("/")).unwrap();
        } else {
            write!(result, "{}", "/").unwrap();
        }

        match component {
            Prefix(_) | RootDir => unreachable!(),
            CurDir => write!(result, ".").unwrap(),
            ParentDir => write!(result, "..").unwrap(),
            Normal(name) => {
                let name = name.to_string_lossy();
                let abbr = if ABBREVIATE_PARENT_DIRECTORIES && components.peek().is_some() {
                    &name[0..1]
                } else {
                    &name[..]
                };
                if styled {
                    write!(result, "{}", Style::default().bold().paint(abbr)).unwrap()
                } else {
                    write!(result, "{}", abbr).unwrap()
                }
            }
        }
    }

    Ok(result)
}

fn style_hostname(hostname: &str) -> String {
    if env::var("SSH_CONNECTION").is_ok() {
        Red.bold().italic().paint(hostname).to_string()
    } else {
        Style::default().italic().paint(hostname).to_string()
    }
}

fn can_write(md: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;

    let user = users::get_current_uid();

    if user == 0 { return true }
    if md.mode() & 0x80 > 0 && user == md.uid() { return true }

    if md.mode() & 0x10 > 0 {
        if let Some(grp) = users::get_group_by_gid(md.gid()) {
            let name = users::get_current_username().unwrap();

            if grp.members.contains(&name) { return true }
        }
    }

    if md.mode() & 0x02 > 0 {
        return true
    }

    false
}

fn format_prompt_char() -> String {
    let user = users::get_current_uid();

    let ch = if user == 0 { "#" } else { "$" };

    match env::current_dir().and_then(|d| d.metadata()) {
        Err(_) => ch.to_string(),
        Ok(md) => {
            if can_write(&md) {
                Green.paint(&ch).to_string()
            } else {
                Red.paint(&ch).to_string()
            }
        }
    }
}

fn get_output(cmd: &mut process::Command) -> Option<String> {
    let output = cmd.output();
    let stdout = match output {
        Ok(process::Output { ref status, ref stdout, .. }) if status.success() => stdout,
        _ => return None
    };

    match std::str::from_utf8(&stdout) {
        Ok(s) => Some(s.trim().to_owned()),
        Err(_) => None
    }
}

fn git_head() -> Result<String> {
    let output = process::Command::new("git").arg("rev-parse").arg("--abbrev-ref").arg("HEAD")
                                             .output();
    let stdout = match output {
        Ok(process::Output { ref status, ref stdout, .. }) if status.success() => stdout,
        _ => return Ok("".into())
    };

    let git_ref = try!(std::str::from_utf8(&stdout)).trim();

    let output = process::Command::new("git").arg("log").arg("-1").arg("--format=%s")
                                             .output();
    let stdout = match output {
        Ok(process::Output { ref status, ref stdout, .. }) if status.success() => stdout,
        _ => return Ok("".into())
    };

    let subject = try!(std::str::from_utf8(&stdout)).trim();

    Ok(format!("({}) \"{}\"", git_ref, subject))
}

fn git_state() -> Result<String> {
    let git_root = match get_output(process::Command::new("git").arg("rev-parse")
                                                                .arg("--git-dir")) {
        Some(x) => x,
        None    => return Ok("".into())
    };

    let git_root = Path::new(&git_root);

    let rebase_merge = git_root.join("rebase-merge").exists();
    let rebase_interactive = rebase_merge && git_root.join("rebase-merge").join("interactive")
                                                     .exists();
    let rebase_apply = git_root.join("rebase-apply").exists();
    let rebase = rebase_apply && git_root.join("rebase-apply").join("rebasing").exists();
    let am = rebase_apply && git_root.join("rebase-apply").join("applying").exists();

    let merge = git_root.join("MERGE_HEAD").exists();
    let cherry_pick = git_root.join("CHERRY_PICK_HEAD").exists();
    let revert = git_root.join("REVERT_HEAD").exists();
    let bisect = git_root.join("BISECT_LOG").exists();

    let mut result = Vec::new();

    if rebase_interactive {
        result.push("REBASE-i");
    } else if rebase_merge {
        result.push("REBASE-m");
    }

    if rebase {
        result.push("REBASE");
    }
    if am {
        result.push("AM");
    }
    if rebase_apply && !(rebase || am) {
        result.push("REBASE/AM");
    }

    if merge {
        result.push("MERGE");
    }
    if cherry_pick {
        result.push("CHERRY-PICK");
    }
    if revert {
        result.push("REVERT");
    }
    if bisect {
        result.push("BISECT");
    }

    let connected = result.connect(" ");

    Ok(
        if result.is_empty() {
            "".into()
        } else if result.len() == 1 {
            Red.bold().paint(&connected).to_string()
        } else {
            Red.bold().reverse().paint(&connected).to_string()
        }
    )
}

fn main() {
    if env::args().any(|x| x == "--title") {
        if let Ok(tab) = env::var("TAB") {
            println!("{}", tab);
        } else if env::var("SSH_CONNECTION").is_ok() {
            println!("SSH {}", get_hostname().unwrap_or_else(|_| "?".into()));
        } else {
            println!("{}", format_path(false).unwrap_or_else(|_| "?".into()));
        }
    } else if env::args().any(|x| x == "--right") {
        println!("{git_head}",
                 git_head = git_head().unwrap_or_else(|_| "".into()));
    } else {
        let hostname = get_hostname().unwrap_or_else(|_| "?".into());
        let username = get_username().unwrap_or_else(|_| "?".into());
        let path = format_path(true).unwrap_or_else(|_| "?".into());

        let state = &[
            git_state().unwrap_or_else(|_| "".into())
        ].connect(" ");
        let state_sep = if state.is_empty() { "" } else { " " };

        println!("{host} {user}:{path} {state}{state_sep}{prompt_char} ",
                host = style_hostname(&hostname),
                user = username,
                path = path,
                state = state, state_sep = state_sep,
                prompt_char = format_prompt_char());
    }
}
