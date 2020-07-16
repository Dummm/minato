# minato

OS-level virtualization tool created as a Bachelor's Degree project.

##### Features
- Containers: create, run, open, list, delete
- Images: pull, list, delete

##### Usage
```
USAGE:
    minato [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -d, --daemon
    -D, --debug
    -e, --exit
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -l, --log-level <log-level>     [env: RUST_LOG=minato]  [default: minato]

SUBCOMMANDS:
    container    Manage containers
    help         Prints this message or the help of the given subcommand(s)
    image        Manage images
```

##### Environment
The program has been developed and tested only on Linux 5.7 and Rust 1.40.

##### Disclamer
Currently, this tool is too unstable for normal use.
Besides that, it could be useful as a starting point for a more advanced virtualization tool or as an educational material on virtualization.