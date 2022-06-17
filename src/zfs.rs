///! zfs
///! Helper-objects for dealing with zfs.  
use chrono::{DateTime, NaiveDateTime, Utc};
use log::{debug, info};
use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::process;

#[derive(Eq, PartialEq, Debug)]
pub struct FS {
    pub name: String,
    date: DateTime<Utc>,
    fs_type: FsType,
    pub snap: bool,
    written: usize,
    fs: String,
}

impl fmt::Display for FS {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "DS {} = {} ({} KB)", self.name, self.snap, self.written)
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
pub enum FsType {
    Filesystem,
    Snapshot,
}

impl FsType {
    fn as_str(&self) -> &'static str {
        match self {
            FsType::Snapshot => "snapshot",
            FsType::Filesystem => "filesystem",
        }
    }
}

impl fmt::Display for FsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Object for working with the zfs-binary. Offers methods to analyse zfs.
///
///
pub struct Zfs {
    executable: String,
    pretend: bool,
    prefix: String,
    option_name: String,
    label: String,
    history: usize,
    timestamp: String,
}

impl Zfs {
    pub fn new<P, L, D>(pretend: bool, prefix: P, label: L, history: usize, date: D) -> Self
    where
        P: Into<String>,
        L: Into<String>,
        D: Into<String>,
    {
        Self {
            executable: default_exec(),
            pretend,
            prefix: prefix.into(),
            option_name: String::from("com.sun:auto-snapshot"),
            label: label.into(),
            history,
            timestamp: date.into(),
        }
    }

    fn cmd(&self) -> process::Command {
        process::Command::new(&self.executable)
    }

    fn sname(&self, name: &str, timestamp: Option<&str>) -> String {
        let Self { prefix, label, .. } = self;
        let ts = match timestamp {
            None => self.timestamp.clone(),
            Some(s) => s.into(),
        };
        format!("{name}@{prefix}_{label}-{ts}")
    }

    fn filter_snaps<'a>(&self, fs: &FS, snaps: &'a [FS]) -> Vec<&'a FS> {
        let name = self.sname(&fs.name, Some(""));
        // filter snaps-list fitting to fs.
        let mut snaps: Vec<&FS> = snaps
            .iter()
            .filter(|&sn| sn.name.strip_prefix(name.as_str()).is_some())
            .collect();
        // Sort descending by FS.date
        debug!("filter snapshots for '{}', found {}", name, snaps.len());
        snaps.sort_unstable_by(|&a, &b| a.date.cmp(&b.date));
        snaps
    }

    /// Returns a list of snapshots to destroy.
    ///
    /// # Arguments
    ///
    /// * fs - filesystem to snap over
    /// * snaps - List of snapshots, to filter and return.
    ///
    pub fn find_expendable_snapshots<'a>(&'a self, fs: &FS, snaps: &'a [FS]) -> Vec<&'a FS> {
        let snaps = self.filter_snaps(fs, snaps);
        if snaps.len() > self.history {
            snaps
                .iter()
                .take(snaps.len() - self.history)
                .copied()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn next_snapshot_needed<'a>(&'a self, min_size: usize, fs: &FS, snaps: &'a [FS]) -> bool {
        let snaps = self.filter_snaps(fs, snaps);
        match snaps.last() {
            Some(&fs) => fs.written > min_size,
            None => true,
        }
    }

    /// Returns a list of the filesystems provided by the local zfs.
    ///
    /// # Arguments
    ///
    /// * fst - filesystem-type
    ///
    pub fn list_filesystems(&self, fst: FsType) -> Vec<FS> {
        let args = [
            "list",
            "-Hp",
            "-o",
            &format!(
                "name,used,{op},{op}:{suf},creation",
                op = &self.option_name,
                suf = &self.label
            ),
            "-t",
            fst.as_str(),
        ];
        info!("zfs {}", args.join(" "));
        let ret = self
            .cmd()
            .args(&args)
            .output()
            .expect("failed to execute process");
        let stdout = String::from_utf8(ret.stdout).unwrap();
        let lines: Vec<FS> = stdout
            .split('\n')
            .filter(|l| !l.is_empty())
            .map(|t| str2fs(t, fst))
            .collect();
        lines
    }

    /// Creates a snapshots.
    ///
    /// # Arguments
    ///
    /// * fs - filesystem to snap over
    ///
    pub fn create_snapshot(&self, fs: &FS) -> Result<(), std::io::Error> {
        let name = self.sname(&fs.name, None);
        let args = &["snapshot", name.as_str()];
        info!("zfs {}", args.join(" "));
        if !self.pretend {
            let output = self.cmd().args(args).output()?;
            debug!("{:?}", output);
        }
        Ok(())
    }

    /// Remove the given filesystem.
    ///
    /// # Arguments
    ///
    /// * fs - filesystem to destroy
    ///
    pub fn remove_snapshot(&self, fs: &FS) -> Result<(), ZfsError> {
        if !fs.snap {
            return Err(ZfsError::InternalError(
                "Filesystems can't be removed!".into(),
            ));
        }
        let args = &["destroy", &fs.name];
        info!("zfs {}", args.join(" "));
        if !self.pretend {
            let output = self.cmd().args(args).output()?;
            debug!("{:?}", output);
        }
        Ok(())
    }
}

pub fn default_exec() -> String {
    env::var_os("ZFS_CMD")
        .unwrap_or_else(|| OsStr::new("zfs").to_owned())
        .to_str()
        .unwrap()
        .to_owned()
}

fn eval(opt: Option<&&str>) -> bool {
    match opt {
        Some(&opt) => opt == "true",
        None => false,
    }
}

fn str2fs(str: &str, fs_type: FsType) -> FS {
    let p: Vec<&str> = str.split('\t').collect();
    let name = p[0].to_string();
    let date = DateTime::from_utc(
        NaiveDateTime::from_timestamp(p[4].parse().unwrap_or_default(), 0),
        Utc,
    );
    FS {
        name: name.clone(),
        written: p[1].parse().unwrap_or_default(),
        snap: eval(p.get(2)) || eval(p.get(3)),
        date,
        fs_type,
        fs: match fs_type {
            FsType::Filesystem => name,
            FsType::Snapshot => name.split('@').next().unwrap().to_string(),
        },
    }
}

pub enum ZfsError {
    IOError(std::io::Error),
    InternalError(String),
}

impl From<std::io::Error> for ZfsError {
    fn from(e: std::io::Error) -> Self {
        ZfsError::IOError(e)
    }
}

impl fmt::Display for ZfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZfsError::IOError(e) => write!(f, "{}", e),
            ZfsError::InternalError(s) => write!(f, "{}", s),
        }
    }
}

mod should {
    use super::*;

    #[test]
    fn parse_zfs_output() {
        let fs = str2fs("tank\t24576\t-\t-\t1608216521", FsType::Filesystem);
        assert_eq!(fs.name, String::from("tank"));
        assert_eq!(fs.written, 24576usize);
        assert!(!fs.snap);
        let fs = str2fs("tank\t24576\t-\ttrue\t1608216521", FsType::Filesystem);
        assert!(fs.snap);
        assert_eq!(
            fs.date,
            DateTime::<Utc>::from_utc(NaiveDateTime::from_timestamp(1608216521, 0), Utc)
        );
        let fs = str2fs("tank\t24576\ttrue\tfalse\t1608216521", FsType::Filesystem);
        assert!(fs.snap);
        let fs = str2fs("tank\t24576\ttrue\ttrue\t1608216521", FsType::Filesystem);
        assert!(fs.snap);
    }

    #[test]
    fn find_expendable_snapshots_enough() {
        let zfs = Zfs::new(true, "zfs-snapshot", "weekly", 1usize, "2019-12-30_1807");
        let fs_snaps = get_snaps();
        let fs_orig = str2fs("tank/SRV/www\t245643\t-\t-\t121212112", FsType::Filesystem);
        let expendables = zfs.find_expendable_snapshots(&fs_orig, &fs_snaps);

        assert_eq!(expendables.len(), 2);
        assert_eq!(
            expendables.first().unwrap().name,
            "tank/SRV/www@zfs-snapshot_weekly-2019-12-30_1207"
        );
        assert_eq!(
            expendables.get(1).unwrap().name,
            "tank/SRV/www@zfs-snapshot_weekly-2019-12-30_1607"
        );
    }

    #[test]
    fn find_expendable_snapshots_to_less() {
        let zfs = Zfs::new(true, "zfs-snapshot", "weekly", 4usize, "2019-12-30_1807");
        let fs_snaps = get_snaps();
        let fs_orig = str2fs("tank/SRV/www\t245643\t-\t-\t121212112", FsType::Filesystem);
        let expendables = zfs.find_expendable_snapshots(&fs_orig, &fs_snaps);

        assert!(expendables.is_empty());
    }

    #[allow(dead_code)]
    fn get_snaps() -> Vec<FS> {
        vec![
            str2fs(
                "tank/SRV/www@zfs-snapshot_weekly-2019-12-30_1207\t23234\t-\t-\t1608216421",
                FsType::Snapshot,
            ),
            str2fs(
                "tank/SRV/www@zfs-snapshot_weekly-2019-12-30_1907\t245643\t-\t-\t1608216921",
                FsType::Snapshot,
            ),
            str2fs(
                "tank/SRV/www@zfs-snapshot_weekly-2019-12-30_1607\t12340\t-\t-\t1608216821",
                FsType::Snapshot,
            ),
        ]
    }
}
