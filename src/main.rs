///! # zfs-snappers
///! A zfs-auto-snapshot like tool written in Rust.
///!
///! License: MIT
///! (c) migmedia 2020 - 2022
mod zfs;

use crate::zfs::{FsType, Zfs};
use chrono::{DateTime, Utc};
use clap::Parser;
use log::{debug, error};
use simplelog::{ColorChoice, CombinedLogger, Config, LevelFilter, TermLogger, TerminalMode};

#[derive(Parser)]
#[clap(version, about, long_about = None)]
struct Opt {
    /// Prints info messages.
    #[clap(short, long)]
    pub verbose: bool,

    /// Prints debug messages.
    #[clap(short, long)]
    pub debug: bool,

    /// Label of snapshot usually 'hourly', 'daily', or 'monthly'.
    pub label: String,

    /// Min size in Kilo-Byte.
    #[clap(short = 'm', long, default_value = "0")]
    pub min_size: usize,

    /// Keeps NUM recent snapshots and destroy older snapshots.
    #[clap(name = "NUM", short, long = "keep", default_value = "8")]
    pub keep: usize,

    /// Prefix of snapshots.
    #[clap(short, long, default_value = "zfs-snappers")]
    pub prefix: String,

    /// Pretending, not really changing anything.
    #[clap(short = 'n', long)]
    pub dry_run: bool,
}

fn main() {
    let opt = Opt::parse();
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
        ColorChoice::Auto,
    )])
    .unwrap();
    let now: DateTime<Utc> = Utc::now();
    let zfs = Zfs::new(
        opt.dry_run,
        opt.prefix,
        opt.label,
        opt.keep,
        now.format("%Y-%m-%d-%H%M").to_string(),
    );
    let snapshots = zfs.list_filesystems(FsType::Snapshot);
    for fs in zfs
        .list_filesystems(FsType::Filesystem)
        .iter()
        .filter(|f| f.snap)
    {
        if !zfs.next_snapshot_needed(opt.min_size, fs, &snapshots) {
            debug!("skip FS: {:?}", fs);
            continue;
        }
        let exp_fs = zfs.find_expendable_snapshots(fs, &snapshots);
        debug!("FS: {:?}", fs);
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
