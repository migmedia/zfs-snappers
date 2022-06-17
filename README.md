# zfs-autosnaprs

A zfs-auto-snapshot like tool written in Rust.

### Work in Progress
Status:
- reads `zfs list`-output and checks every share if its `com.sun:auto-snapshot`-option is set and compares the value with a given `label`.
- creates new snapshots.
- creates new snapshots if the size of its predecessor is below a limit `--min-size` parameter.  
- finds expendable snapshots and destroys they.

```text
zfs-autosnaprs 0.3.0
zfs-auto-snapshot tool written in Rust.

USAGE:
    zfs-autosnaprs [OPTIONS] <LABEL>

ARGS:
    <LABEL>    Label of snapshot usually 'hourly', 'daily', or 'monthly'

OPTIONS:
    -d, --debug                  Prints debug messages
    -h, --help                   Print help information
    -m, --min-size <MIN_SIZE>    Min size in Kilo-Byte [default: 0]
    -n, --dry-run                Pretending, not really changing anything
    -N, --keep <NUM>             Keeps NUM recent snapshots and destroy older snapshots [default: 8]
    -p, --prefix <PREFIX>        Prefix of snapshots [default: zfs-snapshot]
    -v, --verbose                Prints info messages
    -V, --version                Print version information
```

