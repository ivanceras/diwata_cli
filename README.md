# Diwata CLI
This repo is part of [diwata](https://github.com/ivanceras/diwata) project.

This contains a precompiled static html files which you can compile yourself
refer to the main diwata project.


## Quickstart
If you have an existing postgresql database, you can quickly open it using the app by:
```sh
cargo install diwata_cli
diwata_cli --db-url postgres://user:passwd@localhost:5432/dbname -p 8000 --open
```
You can also open sqlite database.
Download this [sqlite sample db](https://github.com/ivanceras/sakila/raw/master/sqlite-sakila-db/sakila.db)
You can then open it by issuing the command
```
diwata_cli --db-url sqlite://sakila.db  -p 80001 --open
```

```
USAGE:
    diwata_cli [OPTIONS]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --address <address>    The address the server would listen, default is 0.0.0.0 [default: 0.0.0.0]
    -u, --db-url <db_url>      Database url to connect to, when set all data is exposed without login needed in the
                               client side
    -p, --port <port>          What port this server would listen to, default is 8000 [default: 8000]
```
