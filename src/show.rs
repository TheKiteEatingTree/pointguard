use crate::settings::Settings;
use ptree::output;
use std::io::{Error, ErrorKind, Result};
use std::path::Path;
use walkdir::{DirEntry, WalkDir};

mod pgtree;
use pgtree::TreeBuilder;

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}

fn display_path(path: &Path) -> Result<String> {
    Ok(path
        .file_stem()
        .ok_or(Error::new(
            ErrorKind::InvalidData,
            "Found a file with no name",
        ))?
        .to_str()
        .ok_or(Error::new(
            ErrorKind::InvalidData,
            "File name is not valid unicode",
        ))?
        .to_string())
}

pub fn show(settings: Settings, input: Option<String>) -> Result<()> {
    // let mut ls = Command::new("ls")
    //     .arg("-R")
    //     .current_dir(settings.dir)
    //     .stdout(Stdio::inherit())
    //     .stderr(Stdio::inherit())
    //     .spawn()?;
    // ls.wait()?;
    // Ok(())
    let path = match &input {
        Some(name) => settings.dir.join(name),
        None => settings.dir,
    };
    if !path.exists() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!(
                "{} is not in the point guard password store.",
                input.unwrap_or(String::from("File or folder"))
            ),
        ));
    }
    let mut builder = TreeBuilder::new(input.unwrap_or(String::from("Point Guard Password Store")));
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
            depth = depth + 1;
        } else {
            if entry.depth() == depth {
                builder.add_empty_child(display_path(path)?);
            } else {
                builder.end_child();
                builder.add_empty_child(display_path(path)?);
                depth = depth - 1;
            }
        }
    }
    let mut root = builder.build();
    root.sort();
    output::print_tree(&root)?;
    Ok(())
}