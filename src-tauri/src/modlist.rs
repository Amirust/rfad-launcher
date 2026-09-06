use std::{
    fs,
    fs::File,
    io::{Error, Write},
    path::Path,
};

const ENABLED: &str = "+RFAD_PATCH";
const DISABLED: &str = "-RFAD_PATCH";

pub fn normalize_patch_entry(content: &str) -> String {
    let (bom, body) = match content.strip_prefix('\u{feff}') {
        Some(body) => ("\u{feff}", body),
        None => ("", content),
    };
    let eol = if body.contains("\r\n") { "\r\n" } else { "\n" };
    let trailing_newline = body.ends_with('\n') || body.is_empty();

    let mut lines: Vec<&str> = body
        .lines()
        .filter(|line| !matches!(line.trim(), ENABLED | DISABLED))
        .collect();

    let at = lines
        .iter()
        .take_while(|line| {
            let t = line.trim();
            t.is_empty() || t.starts_with('#')
        })
        .count();
    lines.insert(at, ENABLED);

    let mut result = format!("{}{}", bom, lines.join(eol));
    if trailing_newline {
        result.push_str(eol);
    }
    result
}

pub fn enable_patch_in_file(path: &Path) -> Result<bool, Error> {
    let content = fs::read_to_string(path)?;
    let new_content = normalize_patch_entry(&content);
    if new_content == content {
        return Ok(false);
    }

    let tmp = path.with_extension("txt.tmp");
    let written = (|| -> Result<(), Error> {
        let mut file = File::create(&tmp)?;
        file.write_all(new_content.as_bytes())?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(true)
}
