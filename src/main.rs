///! # zfs-autosnaprs
///! A zfs-auto-snapshot like tool written in Rust.
///!
///! License: MIT
///! (c) migmedia 2020
extern crate chrono;
extern crate derive_builder;
extern crate log;
extern crate simplelog;
mod zfs;

use crate::zfs::FsType;
use chrono::{DateTime, Utc};
use log::{debug, error};
use simplelog::{CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "zfs-autosnaprs", about = "A zfs snapshot managing utility.")]
struct Opt {
    /// Prints info messages.
    #[structopt(short, long)]
    pub verbose: bool,

    /// Prints debug messages.
    #[structopt(short, long)]
    pub debug: bool,

    /// Label of snapshot usually 'hourly', 'daily', or 'monthly'.
    #[structopt(name = "LAB", short, long = "label")]
    pub label: String,

    /// Min size in Kilo-Byte.
    #[structopt(short = "m", long)]
    pub min_size: Option<usize>,

    /// Keeps NUM recent snapshots and destroy older snapshots.
    #[structopt(name = "NUM", short, long = "keep", default_value = "8")]
    pub keep: usize,

    /// Prefix of snapshots.
    #[structopt(short, long, default_value = "zfs-snapshot")]
    pub prefix: String,

    /// Prints actions without actually doing anything.
    #[structopt(short = "n", long = "dry-run")]
    pub dry_run: bool,
}

fn main() {
    let opt = Opt::from_args();
    CombinedLogger::init(vec![TermLogger::new(
        match opt.debug {
            true => LevelFilter::Debug,
            false => match opt.verbose {
                true => LevelFilter::Info,
                false => LevelFilter::Error,
            },
        },
        Config::default(),
        TerminalMode::Mixed,
    )
    .unwrap()])
    .unwrap();
    let now: DateTime<Utc> = Utc::now();
    let zfs = zfs::ZfsBuilder::default()
        .prefix(opt.prefix)
        .label(opt.label)
        .history(opt.keep)
        .date(format!("{}", now.format("%Y-%m-%d-%H%M")))
        .build()
        .unwrap();
    let snsl = zfs.list_filesystems(FsType::Snapshot);
    for fs in zfs
        .list_filesystems(FsType::Filesystem)
        .iter()
        .filter(|f| f.snap)
    {
        debug!("FS: {}", fs.name);
        let (exp_fs, size) = zfs.find_expendable_snapshots(&fs, &snsl);
        if opt.min_size == None || size > opt.min_size.unwrap() {
            match zfs.create_snapshot(fs) {
                Ok(()) => {
                    debug!("Created! {:?}", &exp_fs);
                    for exp in exp_fs {
                        match zfs.remove_snapshot(exp) {
                            Ok(()) => {}
                            Err(e) => error!("{}", e),
                        };
                    }
                }
                Err(e) => error!("{}", e),
            }
        }
    }
}
