use clap::Parser;
use sqlite::State;
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use zip::read::ZipArchive;
use zip::write::ZipWriter;

const FIELD_SPLIT_CHAR: char = '\x1f';

/// Inflate an Anki deck with pleco links
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to .apkg file to be inflated with pleco links
    #[arg(short, long)]
    file: PathBuf,

    /// Path to output .apkg file
    #[arg(short, long, default_value = "out.apkg")]
    out_file: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let f = File::open(args.file)?;

    let tmp_dir = tempdir()?;
    let mut zip = ZipArchive::new(f)?;

    let mut outfiles: Vec<String> = Vec::new();
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let outfile_name = tmp_dir.path().join(file.name());
        let mut outfile = File::create(&outfile_name)?;
        outfiles.push(file.name().to_string());
        std::io::copy(&mut file, &mut outfile)?;
    }

    for collection in outfiles.iter().filter(|s| s.starts_with("collection")) {
        convert_collection(&tmp_dir.path().join(collection))?;
    }

    let mut zip_writer = ZipWriter::new(File::create(args.out_file)?);
    for outfile in outfiles.iter() {
        zip_writer.start_file(outfile, zip::write::FileOptions::default())?;
        std::io::copy(
            &mut File::open(tmp_dir.path().join(outfile))?,
            &mut zip_writer,
        )?;
    }

    Ok(())
}

fn convert_collection(path: &Path) -> anyhow::Result<()> {
    let conn = sqlite::open(path)?;

    let query = "SELECT id, flds FROM notes;";
    let mut statement = conn.prepare(query)?;
    let mut new_flds: HashMap<i64, String> = HashMap::new();
    while let Ok(State::Row) = statement.next() {
        let id = statement.read::<i64, _>("id")?;
        let flds = statement.read::<String, _>("flds")?;
        // TODO: Don't convert every field, just the answer one
        new_flds.insert(
            id,
            flds.split(|b| b == FIELD_SPLIT_CHAR)
                .map(plecoise)
                .collect::<Vec<String>>()
                .join(&FIELD_SPLIT_CHAR.to_string()), // TODO: this to_string seems sad
        );
    }
    let query = "UPDATE notes SET flds = :flds WHERE id = :id;";
    let mut statement = conn.prepare(query)?;

    for (id, flds) in new_flds.into_iter() {
        statement.bind::<&[(_, sqlite::Value)]>(&[(":flds", flds.into()), (":id", id.into())])?;
        while let State::Row = statement.next()? {}
        statement.reset()?;
    }
    Ok(())
}

/// Replace the hanzi in a given text with Pleco links
fn plecoise(text: &str) -> String {
    let mut segments = text.chars().fold(Vec::new(), |mut acc, c| {
        let current_segment = acc.last_mut();
        let is_hz = cjk::is_simplified_chinese(&c.to_string());
        match current_segment {
            None => {
                if is_hz {
                    acc.push(Segment::Hz(c.to_string()));
                } else {
                    acc.push(Segment::NonHz(c.to_string()));
                }
            }
            Some(Segment::NonHz(ref mut s)) => {
                if is_hz {
                    acc.push(Segment::Hz(c.to_string()));
                } else {
                    s.push(c);
                }
            }
            Some(Segment::Hz(ref mut s)) => {
                if is_hz {
                    s.push(c);
                } else {
                    acc.push(Segment::NonHz(c.to_string()));
                }
            }
        }
        acc
    });

    // Make it idempotent, in a pretty gross manner...
    for i in 0..segments.len() {
        if let Some(Segment::NonHz(s2)) = segments.get(i + 1) {
            if s2.starts_with("</a>") || s2.starts_with(r#"" style"#) {
                if let Some(s1) = segments.get_mut(i) {
                    if let Segment::Hz(inner) = s1 {
                        // Assume that this means that s1 has already been wrapped in a pleco link
                        // Therefore, turn it into a NonHz for idempotency
                        *s1 = Segment::NonHz(inner.clone());
                    }
                }
            }
        }
    }

    segments.into_iter().map(|segment| match segment {
        Segment::NonHz(s) => s,
        Segment::Hz(s) => format!(r#"<a href="plecoapi://x-callback-url/s?q={s}" style="text-decoration:none">{s}</a>"#)
    }).reduce(|s1, s2| format!("{s1}{s2}")).unwrap_or_else(String::new)
}

#[derive(Debug)]
enum Segment {
    NonHz(String),
    Hz(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_plecoise() {
        assert_eq!(plecoise("hello there"), "hello there".to_string());
        assert_eq!(plecoise("hello there, 你好"), r#"hello there, <a href="plecoapi://x-callback-url/s?q=你好" style="text-decoration:none">你好</a>"#.to_string());
        assert_eq!(plecoise("hello there, 你好, how's it going, 你怎么样"), r#"hello there, <a href="plecoapi://x-callback-url/s?q=你好" style="text-decoration:none">你好</a>, how's it going, <a href="plecoapi://x-callback-url/s?q=你怎么样" style="text-decoration:none">你怎么样</a>"#.to_string());
        assert_eq!(plecoise(r#"hello there, <a href="plecoapi://x-callback-url/s?q=你好" style="text-decoration:none">你好</a>"#), r#"hello there, <a href="plecoapi://x-callback-url/s?q=你好" style="text-decoration:none">你好</a>"#.to_string());
        assert_eq!(plecoise(r#"hello there, <a href="plecoapi://x-callback-url/s?q=你好" style="text-decoration:none">你好</a>, how's it going, 你怎么样"#), r#"hello there, <a href="plecoapi://x-callback-url/s?q=你好" style="text-decoration:none">你好</a>, how's it going, <a href="plecoapi://x-callback-url/s?q=你怎么样" style="text-decoration:none">你怎么样</a>"#.to_string());
    }
}
