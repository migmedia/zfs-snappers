# zfs-snappers

A ZFS snapshot handling util. A in rust written alternative to zfs-auto-snapshot.

### Work in Progress
Status:
- reads `zfs list`-output and checks every share if its `com.sun:auto-snapshot`-option is set and compares the value with a given `label`.
- creates new snapshots.
- creates new snapshots if the size of its predecessor is below a limit `--min-size` parameter.  
- finds expendable snapshots and destroys they.

```text
zfs-snappers 0.3.1
ZFS snapshot handling util.

USAGE:
    zfs-snappers [OPTIONS] <LABEL>

ARGS:
    <LABEL>    Label of snapshot usually 'hourly', 'daily', or 'monthly'

OPTIONS:
    -d, --debug                  Prints debug messages
    -h, --help                   Print help information
    -m, --min-size <MIN_SIZE>    Min size in Kilo-Byte [default: 0]
    -n, --dry-run                Pretending, not really changing anything
    -N, --keep <NUM>             Keeps NUM recent snapshots and destroy older snapshots [default: 8]
    -p, --prefix <PREFIX>        Prefix of snapshots [default: zfs-snappers]
    -v, --verbose                Prints info messages
    -V, --version                Print version information
```

