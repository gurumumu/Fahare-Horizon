use std::{fs, io, path::{Path, PathBuf}, time::{SystemTime, UNIX_EPOCH}};

fn invalid(message: &str) -> io::Error { io::Error::new(io::ErrorKind::InvalidInput, message) }

// The engine supplies the exact active save. Never enumerate or delete worlds.
pub fn archive(world: &Path) -> io::Result<PathBuf> {
    if !world.is_absolute() { return Err(invalid("world path must be absolute")); }
    let meta = fs::symlink_metadata(world)?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(invalid("world is not a real directory"));
    }
    let resolved = world.canonicalize()?;
    let parent = resolved.parent().ok_or_else(|| invalid("world has no parent"))?;
    let name = resolved.file_name().ok_or_else(|| invalid("world has no name"))?;
    let level = fs::symlink_metadata(resolved.join("level.dat"))?;
    if !level.is_file() || level.file_type().is_symlink() {
        return Err(invalid("missing regular level.dat"));
    }
    let epoch = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|_| invalid("clock before epoch"))?.as_nanos();
    // Reserve a unique sibling directory atomically. Rename on the same volume.
    // If rename fails, leave the original save exactly where it was.
    for i in 0..100 {
        let mut label = name.to_os_string();
        label.push(format!(".fahare-backup-{epoch}-{}-{i}", std::process::id()));
        let reserve = parent.join(label);
        match fs::create_dir(&reserve) {
            Ok(()) => {
                let destination = reserve.join(name);
                fs::rename(&resolved, &destination)?;
                return Ok(destination);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists, "no unused backup directory"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn archives_entire_save_without_deleting() {
        let root = std::env::temp_dir().join(format!("fahare-test-{}-{}", std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        let world = root.join("world");
        fs::create_dir_all(world.join("playerdata")).unwrap();
        fs::write(world.join("level.dat"), b"test").unwrap();
        fs::write(world.join("playerdata/a.dat"), b"player").unwrap();
        let archive = archive(&world).unwrap();
        assert!(!world.exists());
        assert_eq!(fs::read(archive.join("playerdata/a.dat")).unwrap(), b"player");
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn refuses_incomplete_save() {
        let root = std::env::temp_dir().join(format!("fahare-bad-{}-{}", std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()));
        fs::create_dir_all(&root).unwrap();
        assert!(archive(&root).is_err());
        assert!(root.exists());
        fs::remove_dir(root).unwrap();
    }
}
