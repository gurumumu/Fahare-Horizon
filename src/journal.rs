//! Durable world-local death journal. Corrupt or inaccessible state fails closed.
use std::{fs::{self, OpenOptions}, io::{self, Read, Write}, path::{Path, PathBuf}};
use crate::round::{Round, Rules};

const LIMIT: u64 = 16 * 1024 * 1024;
const FILE: &str = "fahare-deaths-v1.log";
fn bad(message: &str) -> io::Error { io::Error::new(io::ErrorKind::InvalidData, message) }

pub struct Journal { path: PathBuf }

impl Journal {
    pub fn open(world: &Path, rules: Rules) -> io::Result<(Self, Round)> {
        let path = world.join(FILE);
        let mut round = Round::default();
        round.rules = rules;
        match fs::symlink_metadata(&path) {
            Ok(m) if m.is_file() && !m.file_type().is_symlink() && m.len() <= LIMIT => {}
            Ok(_) => return Err(bad("invalid Fahare journal file")),
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                OpenOptions::new().write(true).create_new(true).open(&path)?.sync_all()?;
                return Ok((Self {path}, round));
            }
            Err(e) => return Err(e),
        }
        let mut bytes = Vec::new();
        fs::File::open(&path)?.take(LIMIT+1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > LIMIT || (!bytes.is_empty() && bytes.last() != Some(&b'\n')) {
            return Err(bad("truncated or oversized Fahare journal"));
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| bad("invalid journal encoding"))?;
        for line in text.lines() {
            if line.len() != 32 || !line.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(bad("invalid journal record"));
            }
            round.replay_death(u128::from_str_radix(line, 16).map_err(|_| bad("invalid UUID"))?);
        }
        Ok((Self {path}, round))
    }

    pub fn append(&self, id: u128) -> io::Result<()> {
        let meta = fs::symlink_metadata(&self.path)?;
        if !meta.is_file() || meta.file_type().is_symlink() || meta.len() + 33 > LIMIT {
            return Err(bad("unsafe or full Fahare journal"));
        }
        let mut f = OpenOptions::new().append(true).open(&self.path)?;
        f.write_all(format!("{id:032x}\n").as_bytes())?;
        f.sync_data()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp() -> PathBuf {
        let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let p = std::env::temp_dir().join(format!("fahare-journal-{}-{n}",std::process::id()));
        fs::create_dir(&p).unwrap();
        p
    }
    #[test] fn survives_reopen() {
        let p = temp();
        let (j, _) = Journal::open(&p, Rules::default()).unwrap();
        j.append(123).unwrap();
        j.append(123).unwrap();
        let (_, r) = Journal::open(&p, Rules::default()).unwrap();
        assert_eq!(r.deaths(123), 2);
        fs::remove_dir_all(p).unwrap();
    }
    #[test] fn partial_record_refused() {
        let p = temp();
        fs::write(p.join(FILE), b"123").unwrap();
        assert!(Journal::open(&p, Rules::default()).is_err());
        fs::remove_dir_all(p).unwrap();
    }
}
