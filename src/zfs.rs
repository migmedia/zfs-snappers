///! zfs
///! Helper-objects for dealing with zfs.  
use derive_builder::*;
use log::{debug, info};
use std::env;
use std::ffi::OsStr;
use std::fmt;
use std::process;

#[derive(Eq, PartialEq, Debug)]
pub struct FS {
    pub name: String,
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
    fn to_str(&self) -> &str {
        match self {
            FsType::Snapshot => "snapshot",
            FsType::Filesystem => "filesystem",
        }
    }
}

impl fmt::Display for FsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_str())
    }
}

/// Object for working with the zfs-binary. Offers methods to analyse zfs.
///
///
#[derive(Builder)]
#[builder(setter(into))]
pub struct Zfs {
    #[builder(default = "self.default_exec()")]
    executable: String,
    #[builder(default = "false")]
    pretend: bool,
    #[builder(default = "self.default_prefix()")]
    prefix: String,
    #[builder(default = "self.default_option_name()")]
    option_name: String,
    label: String,
    history: usize,
    date: String,
}

impl Zfs {
    fn cmd(&self) -> process::Command {
        process::Command::new(&self.executable)
    }

    /// Returns a list of snapshots to destroy.
    ///
    /// # Arguments
    ///
    /// * fs - filesystem to snap over
    /// * snaps - List of snapshots, to filter and return.
    ///
    pub fn find_expendable_snapshots<'a>(
        &'a self,
        fs: &FS,
        snaps: &'a Vec<FS>,
    ) -> (Vec<&FS>, usize) {
        let name = format!("{}@{}-{}", fs.name, self.prefix, self.label);

        // filter snaps-list fitting to fs.
        let mut snaps: Vec<&FS> = snaps.iter().filter(|&sn| sn.fs == name).collect();

        // Sort descending by FS.name
        snaps.sort_unstable_by(|&a, &b| b.name.cmp(&a.name));

        let size = match snaps.iter().next() {
            None => usize::max_value(),
            Some(&fs) => fs.written,
        };

        (
            // skip amount to hold, return the rest.
            snaps.iter().skip(self.history).map(|t| t.clone()).collect(),
            size,
        )
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
                "name,used,{op},{op}:{suf}",
                op = &self.option_name,
                suf = &self.label
            ),
            "-t",
            fst.to_str(),
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
            .filter(|l| l.len() > 0)
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
        let args = &[
            "snapshot",
            &format!("{}@{}-{}_{}", fs.name, self.prefix, self.label, self.date),
        ];
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
    pub fn remove_snapshot(&self, fs: &FS) -> Result<(), std::io::Error> {
        let args = &["destroy", &fs.name];
        info!("zfs {}", args.join(" "));
        if !self.pretend {
            let output = self.cmd().args(args).output()?;
            debug!("{:?}", output);
        }
        Ok(())
    }
}

impl ZfsBuilder {
    fn default_exec(&self) -> String {
        env::var_os("ZFS_CMD")
            .unwrap_or(OsStr::new("zfs").to_owned())
            .to_str()
            .unwrap()
            .to_owned()
    }

    fn default_option_name(&self) -> String {
        String::from("com.sun:auto-snapshot")
    }

    fn default_prefix(&self) -> String {
        String::from("zfs-snapshot")
    }
}

fn eval(opt: Option<&&str>) -> bool {
    match opt {
        Some(&opt) => match opt {
            "true" => true,
            _ => false,
        },
        None => false,
    }
}

fn str2fs(str: &str, fst: FsType) -> FS {
    let p: Vec<&str> = str.split("\t").collect();
    let name = p[0].to_string();
    FS {
        name: name.clone(),
        written: p[1].parse().unwrap(),
        snap: eval(p.get(2)) || eval(p.get(3)),
        fs_type: fst,
        fs: match fst {
            FsType::Filesystem => String::from(name),
            FsType::Snapshot => name.split("_").next().unwrap().to_string(),
        },
    }
}

#[test]
fn test_str2fs() {
    let fs = str2fs("tank\t24576\t-\t-".into(), FsType::Filesystem);
    assert_eq!(fs.name, String::from("tank"));
    assert_eq!(fs.written, 24576usize);
    assert_eq!(fs.snap, false);
    let fs = str2fs("tank\t24576\t-\ttrue".into(), FsType::Filesystem);
    assert_eq!(fs.snap, true);
    let fs = str2fs("tank\t24576\ttrue\tfalse".into(), FsType::Filesystem);
    assert_eq!(fs.snap, true);
    let fs = str2fs("tank\t24576\ttrue\ttrue".into(), FsType::Filesystem);
    assert_eq!(fs.snap, true);
}

#[test]
fn test_zfs_find_expendable_snapshots() {
    let zfs = ZfsBuilder::default()
        .label("weekly")
        .history(1usize)
        .date(String::from("2019-12-30_1807"))
        .build()
        .unwrap();
    let fs_snaps = vec![
        str2fs(
            "tank/SRV/www@zfs-snapshot-weekly_2019-12-30_1207\t23234\t-\t-".into(),
            FsType::Snapshot,
        ),
        str2fs(
            "tank/SRV/www@zfs-snapshot-weekly_2019-12-30_1407\t245643\t-\t-".into(),
            FsType::Snapshot,
        ),
        str2fs(
            "tank/SRV/www@zfs-snapshot-weekly_2019-12-30_1607\t12340\t-\t-".into(),
            FsType::Snapshot,
        ),
    ];
    let fs_orig = str2fs("tank/SRV/www\t245643\t-\t-".into(), FsType::Filesystem);
    let (fs, size) = zfs.find_expendable_snapshots(&fs_orig, &fs_snaps);

    assert_eq!(size, 12340usize);
}
