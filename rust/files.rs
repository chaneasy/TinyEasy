use crate::{
    error::Error,
    model::{ImageItem, SaveMode, SaveOptions, Status},
};
use iced::widget::image::Handle;
use std::{
    collections::HashSet,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};
use tokio::sync::mpsc;

pub fn supported(path: &Path) -> bool {
    path.extension().and_then(|s| s.to_str()).is_some_and(|s| {
        matches!(
            s.to_ascii_lowercase().as_str(),
            "jpg" | "jpeg" | "png" | "webp"
        )
    })
}
#[derive(Debug, Clone)]
pub enum ImportEvent {
    Batch(Vec<ImageItem>),
    Error(Error),
    Finished,
}
/// Directory traversal and header reads happen exclusively on a blocking worker.
pub fn scan(paths: Vec<PathBuf>, known: HashSet<PathBuf>, tx: mpsc::Sender<ImportEvent>) {
    let mut seen = known;
    let mut batch = Vec::with_capacity(32);
    for root in paths {
        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .sort_by_file_name()
        {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => {
                    let _ = tx.blocking_send(ImportEvent::Error(Error::File));
                    continue;
                }
            };
            if !entry.file_type().is_file() || !supported(entry.path()) {
                continue;
            }
            let path = match entry.path().canonicalize() {
                Ok(p) => p,
                Err(_) => {
                    let _ = tx.blocking_send(ImportEvent::Error(Error::File));
                    continue;
                }
            };
            if !seen.insert(path.clone()) {
                continue;
            }
            let metadata = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => {
                    let _ = tx.blocking_send(ImportEvent::Error(Error::File));
                    continue;
                }
            };
            let (width, height) = image::image_dimensions(&path).unwrap_or((0, 0));
            batch.push(ImageItem {
                id: 0,
                revision: 0,
                path,
                bytes: metadata.len(),
                width,
                height,
                thumbnail: None,
                thumbnail_requested: false,
                status: Status::Pending,
                progress: 0.,
                result: None,
            });
            if batch.len() == 32
                && tx
                    .blocking_send(ImportEvent::Batch(std::mem::take(&mut batch)))
                    .is_err()
            {
                return;
            }
        }
    }
    if !batch.is_empty() {
        let _ = tx.blocking_send(ImportEvent::Batch(batch));
    }
    let _ = tx.blocking_send(ImportEvent::Finished);
}
pub fn decode(path: &Path, thumbnail: bool) -> Result<Handle, Error> {
    let mut reader = image::ImageReader::open(path)
        .map_err(|_| Error::File)?
        .with_guessed_format()
        .map_err(|_| Error::Image)?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(256 * 1024 * 1024);
    limits.max_image_width = Some(24000);
    limits.max_image_height = Some(24000);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|e| {
        if matches!(e, image::ImageError::Limits(_)) {
            Error::ImageLimit
        } else {
            Error::Image
        }
    })?;
    let rgba = if thumbnail {
        decoded.thumbnail(180, 180).to_rgba8()
    } else {
        decoded.to_rgba8()
    };
    Ok(Handle::from_rgba(
        rgba.width(),
        rgba.height(),
        rgba.into_raw(),
    ))
}
pub fn common_root(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut root = paths.first()?.parent()?.to_owned();
    for path in paths.iter().skip(1) {
        let parent = path.parent()?;
        while !parent.starts_with(&root) {
            if !root.pop() {
                return None;
            }
        }
    }
    Some(root)
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    len: u64,
    modified: Option<SystemTime>,
}
pub fn fingerprint(path: &Path) -> Result<Fingerprint, Error> {
    let m = std::fs::symlink_metadata(path)?;
    if !m.is_file() || m.file_type().is_symlink() {
        return Err(Error::UnsafePath);
    }
    Ok(Fingerprint {
        len: m.len(),
        modified: m.modified().ok(),
    })
}
/// An unpublished file is removed automatically on every error path.
pub struct Destination {
    temp: tempfile::NamedTempFile,
    input: PathBuf,
    output_dir: PathBuf,
    overwrite: bool,
    original: Fingerprint,
}
impl Destination {
    pub fn prepare(input: &Path, options: &SaveOptions) -> Result<Self, Error> {
        let original = fingerprint(input)?;
        let overwrite = options.mode == SaveMode::Overwrite;
        let output_dir = if overwrite {
            input.parent().ok_or(Error::File)?.to_owned()
        } else {
            let selected = options
                .directory
                .as_ref()
                .ok_or(Error::UnsafePath)?
                .canonicalize()?;
            let relative = if options.preserve {
                options
                    .common_root
                    .as_ref()
                    .and_then(|r| input.parent()?.strip_prefix(r).ok())
                    .unwrap_or(Path::new(""))
            } else {
                Path::new("")
            };
            if relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
            {
                return Err(Error::UnsafePath);
            }
            let mut directory = selected.clone();
            // Reject symlinks before creating descendants, rather than checking after creation.
            for part in relative.components() {
                directory.push(part);
                match std::fs::symlink_metadata(&directory) {
                    Ok(m) if m.file_type().is_symlink() || !m.is_dir() => {
                        return Err(Error::UnsafePath);
                    }
                    Ok(_) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        std::fs::create_dir(&directory)?
                    }
                    Err(_) => return Err(Error::File),
                }
            }
            let canonical = directory.canonicalize()?;
            if !canonical.starts_with(&selected) {
                return Err(Error::UnsafePath);
            }
            canonical
        };
        let temp = tempfile::Builder::new()
            .prefix(".tinyeasy-")
            .tempfile_in(&output_dir)?;
        Ok(Self {
            temp,
            input: input.to_owned(),
            output_dir,
            overwrite,
            original,
        })
    }
    pub fn path(&self) -> &Path {
        self.temp.path()
    }
    pub fn commit(self) -> Result<PathBuf, Error> {
        self.temp.as_file().sync_all()?;
        if fingerprint(&self.input)? != self.original {
            return Err(Error::OriginalChanged);
        }
        if self.overwrite {
            self.temp
                .as_file()
                .set_permissions(std::fs::metadata(&self.input)?.permissions())?;
            self.temp.persist(&self.input).map_err(|_| Error::File)?;
            Ok(self.input)
        } else {
            let mut temp = self.temp;
            let stem = self.input.file_stem().ok_or(Error::UnsafePath)?;
            let extension = self.input.extension().ok_or(Error::UnsafePath)?;
            for number in 0..100_000 {
                let mut name = stem.to_os_string();
                name.push(".min");
                if number > 0 {
                    name.push(format!("-{number}"));
                }
                name.push(".");
                name.push(extension);
                let output = self.output_dir.join(name);
                match temp.persist_noclobber(&output) {
                    Ok(_) => return Ok(output),
                    Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => temp = e.file,
                    Err(_) => return Err(Error::File),
                }
            }
            Err(Error::File)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    fn options(dir: &Path) -> SaveOptions {
        SaveOptions {
            mode: SaveMode::Directory,
            directory: Some(dir.to_owned()),
            common_root: None,
            preserve: false,
        }
    }
    #[test]
    fn unicode_collision_does_not_overwrite_unrelated_file() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("熊 猫.png");
        std::fs::write(&input, b"original").unwrap();
        std::fs::write(dir.path().join("熊 猫.min.png"), b"unrelated").unwrap();
        let mut dest = Destination::prepare(&input, &options(dir.path())).unwrap();
        dest.temp.write_all(b"result").unwrap();
        assert_eq!(
            dest.commit().unwrap(),
            dir.path().canonicalize().unwrap().join("熊 猫.min-1.png")
        );
        assert_eq!(
            std::fs::read(dir.path().join("熊 猫.min.png")).unwrap(),
            b"unrelated"
        );
    }
    #[test]
    fn failed_overwrite_removes_temporary_file_and_keeps_original() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.png");
        std::fs::write(&input, b"original").unwrap();
        {
            let mut o = options(dir.path());
            o.mode = SaveMode::Overwrite;
            let d = Destination::prepare(&input, &o).unwrap();
            std::fs::write(d.path(), b"partial").unwrap();
        }
        assert_eq!(std::fs::read(&input).unwrap(), b"original");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[test]
    fn changed_original_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("a.png");
        std::fs::write(&input, b"original").unwrap();
        let mut o = options(dir.path());
        o.mode = SaveMode::Overwrite;
        let d = Destination::prepare(&input, &o).unwrap();
        std::fs::write(&input, b"changed data").unwrap();
        assert_eq!(d.commit().unwrap_err(), Error::OriginalChanged);
        assert_eq!(std::fs::read(&input).unwrap(), b"changed data");
    }
    #[cfg(unix)]
    #[test]
    fn symlink_cannot_escape_output_directory() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let source = tempfile::tempdir().unwrap();
        let input = source.path().join("nested/a.png");
        std::fs::create_dir(input.parent().unwrap()).unwrap();
        std::fs::write(&input, b"x").unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("nested")).unwrap();
        let o = SaveOptions {
            common_root: Some(source.path().to_owned()),
            preserve: true,
            ..options(dir.path())
        };
        assert!(matches!(
            Destination::prepare(&input, &o),
            Err(Error::UnsafePath)
        ));
        assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
    }
    #[test]
    fn scan_is_recursive_and_deduplicates_canonical_paths() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("中文 空格");
        std::fs::create_dir(&nested).unwrap();
        let path = nested.join("a.PNG");
        std::fs::write(&path, b"image").unwrap();
        let (tx, mut rx) = mpsc::channel(8);
        scan(vec![dir.path().to_owned(), path], HashSet::new(), tx);
        let mut count = 0;
        while let Ok(e) = rx.try_recv() {
            if let ImportEvent::Batch(batch) = e {
                count += batch.len();
            }
        }
        assert_eq!(count, 1);
    }
}

#[cfg(test)]
mod safety_tests {
    use super::*;
    #[test]
    fn simultaneous_same_name_outputs_are_unique() {
        let source = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let input = source.path().join("same.webp");
        std::fs::write(&input, b"original").unwrap();
        let options = SaveOptions {
            mode: SaveMode::Directory,
            directory: Some(output.path().to_owned()),
            common_root: None,
            preserve: false,
        };
        let a = Destination::prepare(&input, &options).unwrap();
        let b = Destination::prepare(&input, &options).unwrap();
        std::fs::write(a.path(), b"first").unwrap();
        std::fs::write(b.path(), b"second").unwrap();
        let task = std::thread::spawn(move || a.commit().unwrap());
        let second = b.commit().unwrap();
        let first = task.join().unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read(input).unwrap(), b"original");
        assert_eq!(std::fs::read_dir(output.path()).unwrap().count(), 2);
    }
    #[test]
    fn preserved_structure_uses_shared_parent_without_path_traversal() {
        let source = tempfile::tempdir().unwrap();
        let output = tempfile::tempdir().unwrap();
        let paths = [
            source.path().join("one/熊 猫.png"),
            source.path().join("two/熊 猫.png"),
        ];
        for path in &paths {
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, b"original").unwrap();
        }
        let options = SaveOptions {
            mode: SaveMode::Directory,
            directory: Some(output.path().to_owned()),
            common_root: common_root(&paths),
            preserve: true,
        };
        let destination = Destination::prepare(&paths[0], &options).unwrap();
        std::fs::write(destination.path(), b"compressed").unwrap();
        let path = destination.commit().unwrap();
        assert!(path.ends_with("one/熊 猫.min.png"));
        assert!(path.starts_with(output.path().canonicalize().unwrap()));
    }
    #[test]
    fn dimensions_thumbnail_and_full_preview_decode_on_sample_copies() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("中文 空格.png");
        image::RgbaImage::from_pixel(800, 400, image::Rgba([23, 107, 135, 255]))
            .save(&path)
            .unwrap();
        let thumb = decode(&path, true).unwrap();
        let preview = decode(&path, false).unwrap();
        assert!(matches!(
            thumb,
            Handle::Rgba {
                width: 180,
                height: 90,
                ..
            }
        ));
        assert!(matches!(
            preview,
            Handle::Rgba {
                width: 800,
                height: 400,
                ..
            }
        ));
    }
    #[cfg(unix)]
    #[test]
    fn unwritable_destination_preserves_input() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("original.png");
        std::fs::write(&input, b"original").unwrap();
        let options = SaveOptions {
            mode: SaveMode::Overwrite,
            directory: None,
            common_root: None,
            preserve: false,
        };
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o500)).unwrap();
        let result = Destination::prepare(&input, &options);
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(matches!(result, Err(Error::File)));
        assert_eq!(std::fs::read(&input).unwrap(), b"original");
    }
}

#[cfg(test)]
mod performance {
    use super::*;
    #[test]
    #[ignore = "run explicitly with cargo test --release background_import_benchmark -- --ignored --nocapture"]
    fn background_import_benchmark() {
        let dir = tempfile::tempdir().unwrap();
        let sample = dir.path().join("sample.png");
        image::RgbaImage::from_pixel(180, 120, image::Rgba([23, 107, 135, 255]))
            .save(&sample)
            .unwrap();
        for index in 0..999 {
            std::fs::copy(&sample, dir.path().join(format!("image-{index}.png"))).unwrap();
        }
        let (tx, mut rx) = mpsc::channel(4);
        let path = dir.path().to_owned();
        let start = std::time::Instant::now();
        let worker = std::thread::spawn(move || scan(vec![path], HashSet::new(), tx));
        let mut count = 0;
        let mut first_page = vec![];
        while let Some(event) = rx.blocking_recv() {
            if let ImportEvent::Batch(batch) = event {
                count += batch.len();
                if first_page.len() < 18 {
                    first_page.extend(batch.into_iter().take(18 - first_page.len()));
                }
            }
        }
        worker.join().unwrap();
        let scan_ms = start.elapsed().as_secs_f64() * 1000.;
        for item in first_page {
            decode(&item.path, true).unwrap();
        }
        println!(
            "1000 generated 180x120 PNGs: scan+dimensions={scan_ms:.2}ms; including 18 thumbnail decodes={:.2}ms",
            start.elapsed().as_secs_f64() * 1000.
        );
        assert_eq!(count, 1000);
    }
}
