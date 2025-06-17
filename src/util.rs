use crate::strerr::Strerr;
use std::{
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

pub fn collect_paths<P>(path: P, extension: &str) -> Result<Vec<PathBuf>, String>
where
    P: AsRef<Path>,
{
    let body = |path: PathBuf| {
        if path.is_file() {
            path.extension()
                .map(|str| str == extension)
                .and_then(|ext_matched| if ext_matched { Some(vec![path]) } else { None })
        } else if path.is_dir() {
            collect_paths(path, extension).ok()
        } else {
            None
        }
    };

    Ok(std::fs::read_dir(path)
        .strerr()?
        .filter_map(|entry_result| body(entry_result.ok()?.path()))
        .flatten()
        .collect())
}

pub fn find_image<P>(path: P) -> Option<PathBuf>
where
    P: AsRef<Path>,
{
    use std::fs;

    let osu_path = fs::read_dir(path.as_ref())
        .ok()?
        .filter_map(|entry_result| entry_result.ok().map(|entry| entry.path()))
        .find(|path| path.extension().map(|str| str == "osu").unwrap_or(false))?;

    let file = fs::File::open(osu_path).ok()?;

    let image_path = BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .skip_while(|line| line != "[Events]" || line.starts_with("//"))
        .filter_map(|line| {
            line.split(',')
                .nth(2)
                .map(|elt| elt.trim_matches('\"').to_string())
        })
        .find(|name| {
            let lc = name.to_lowercase();
            lc.ends_with("png") || lc.ends_with("jpg")
        })?;

    Some(path.as_ref().join(image_path))
}
