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
