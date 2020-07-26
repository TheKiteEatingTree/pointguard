use crate::error::{PointGuardError, Result};
use crate::gpg;
use crate::opts::Show;
use crate::settings::Settings;
use anyhow::anyhow;
use ptree::output;
use std::{
    env,
    io::{self, Write},
    path::Path,
    process::{Command, Stdio},
};
use walkdir::{DirEntry, WalkDir};

mod pgtree;
use pgtree::TreeBuilder;

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn display_path(path: &Path) -> Result<String> {
    Ok(path
        .file_stem()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Found a file with no name"))?
        .to_str()
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "File name is not valid unicode")
        })?
        .to_string())
}

fn print_tree(buffer: &mut dyn io::Write, path: &Path, input: Option<String>) -> Result<()> {
    let mut builder =
        TreeBuilder::new(input.unwrap_or_else(|| String::from("Point Guard Password Store")));
    let walker = WalkDir::new(&path).into_iter();
    let mut depth = 1;
    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let entry = match entry {
            Ok(entry) => entry,
            // TODO: should this return an error?
            Err(_e) => continue,
        };
        if entry.depth() == 0 {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            builder.begin_child(display_path(path)?);
            depth += 1;
        } else if entry.depth() == depth {
            builder.add_empty_child(display_path(path)?);
        } else {
            builder.end_child();
            builder.add_empty_child(display_path(path)?);
            depth -= 1;
        }
    }
    let mut root = builder.build();
    root.sort();
    output::write_tree(&root, buffer)?;
    Ok(())
}

pub fn show(buffer: &mut dyn io::Write, opts: Show, settings: Settings) -> Result<()> {
    let (path, file) = match &opts.input {
        Some(name) => (
            settings.dir.join(name),
            settings.dir.join(name.to_owned() + ".gpg"),
        ),
        None => (settings.dir.clone(), settings.dir),
    };
    if file.exists() && !file.is_dir() {
        let pw = gpg::decrypt(&file)?;
        if opts.clip {
            let exe = env::current_exe()?;
            let mut child = Command::new(exe)
                .arg("clip")
                .stdin(Stdio::piped())
                .spawn()?;
            let child_stdin = child.stdin.as_mut();
            let child_stdin = child_stdin.ok_or_else(|| {
                PointGuardError::Other(anyhow!("Error launching child to copy to clipboard."))
            })?;
            let pw = pw.lines().next().ok_or_else(|| 
                PointGuardError::Other(anyhow!("Error reading the line from the password file."))
            )?;
            child_stdin.write_all(pw.as_bytes())?;
            writeln!(
                buffer,
                "Copied {} to clipboard. Will clear in {} seconds.",
                opts.input.unwrap(),
                settings.clip_time
            )?;
            Ok(())
        } else {
            write!(buffer, "{}", pw)?;
            Ok(())
        }
    } else if path.is_dir() {
        print_tree(buffer, &path, opts.input)
    } else {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "{} is not in the point guard password store.",
                opts.input.unwrap_or_else(|| String::from("File or folder"))
            ),
        )
        .into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn get_test_settings() -> Settings {
        Settings {
            dir: PathBuf::from("test-store-enc"),
            clip_time: 45,
            generated_length: 25,
            editor: String::from("vim"),
        }
    }

    #[test]
    fn print_password() {
        let mut result: Vec<u8> = vec![];
        show(
            &mut result,
            Show::new(Some(String::from("test"))),
            get_test_settings(),
        )
        .unwrap();
        assert_eq!(String::from_utf8(result).unwrap().trim(), "test");
    }

    #[test]
    fn print_website_password() {
        let mut result: Vec<u8> = vec![];
        show(
            &mut result,
            Show::new(Some(String::from("pointguard.dev"))),
            get_test_settings(),
        )
        .unwrap();
        assert_eq!(String::from_utf8(result).unwrap().trim(), "pointguard.dev");
    }

    #[test]
    fn print_password_with_same_name_as_dir() {
        let mut result: Vec<u8> = vec![];
        show(
            &mut result,
            Show::new(Some(String::from("dir"))),
            get_test_settings(),
        )
        .unwrap();
        assert_eq!(String::from_utf8(result).unwrap().trim(), "dir");
    }

    #[test]
    fn print_password_in_dir() {
        let mut result: Vec<u8> = vec![];
        show(
            &mut result,
            Show::new(Some(String::from("dir/test"))),
            get_test_settings(),
        )
        .unwrap();
        assert_eq!(String::from_utf8(result).unwrap().trim(), "dir/test");
    }

    #[test]
    fn print_root_tree() {
        let mut result: Vec<u8> = vec![];
        show(&mut result, Show::new(None), get_test_settings()).unwrap();
        let result_string = String::from_utf8(result).unwrap();
        assert!(result_string.contains("test"));
        assert!(result_string.contains("pointguard.dev"));
        assert!(result_string.contains("dir"));
        assert!(result_string.contains("unique"));
        assert!(!result_string.contains("notinstore"));
    }

    #[test]
    fn print_tree_with_same_name_as_password() {
        let mut result: Vec<u8> = vec![];
        show(
            &mut result,
            Show::new(Some(String::from("dir/"))),
            get_test_settings(),
        )
        .unwrap();
        let result_string = String::from_utf8(result).unwrap();
        assert!(result_string.contains("test"));
        assert!(result_string.contains("unique"));
        assert!(result_string.contains("dir"));
        assert!(!result_string.contains("notinstore"));
    }
}
