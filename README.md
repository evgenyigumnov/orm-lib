# Ormlib

Indeed, an ORM library, not a framework, written in Rust

## Features

The main idea that I put into my ORM library is a minimum of stupid code and easy use of the library. I wanted users not to have to write long chains of function calls to construct a simple SQL query.

- [x] SQLite support
- [x] MySQL support

## Usage

Cargo.toml

```toml
[dependencies]
ormlib = {version = "1.0.0", features = ["sqlite"]} # or "mysql"
ormlib_derive = "1.0.0"
```

```rust,ignore
{{#include ./test/src/main.rs:readme_example}}
```


Example output:

```text 
[2023-09-08T13:33:22Z DEBUG ormlib] "CREATE TABLE user (id INTEGER PRIMARY KEY AUTOINCREMENT, name  TEXT,age INTEGER)"
[2023-09-08T13:33:22Z DEBUG ormlib] "insert into user (name,age) values (\"John\",30)"
[2023-09-08T13:33:22Z DEBUG ormlib] "select * from user where rowid = 1"
[2023-09-08T13:33:22Z DEBUG ormlib] "insert into user (name,age) values (\"Mary\",30)"
[2023-09-08T13:33:22Z DEBUG ormlib] "select * from user where rowid = 2"
[2023-09-08T13:33:22Z DEBUG ormlib] "select * from user where id = 1"
[2023-09-08T13:33:22Z DEBUG test::tests] User = Some(User { id: 1, name: Some("John"), age: 30 })
[2023-09-08T13:33:22Z DEBUG ormlib] "select * from user"
[2023-09-08T13:33:22Z DEBUG test::tests] Users = [User { id: 1, name: Some("John"), age: 30 }, User { id: 2, name: Some("Mary"), age: 30 }]
[2023-09-08T13:33:22Z DEBUG ormlib] "update user set name = \"Mike\",age = 30 where id = 1"
[2023-09-08T13:33:22Z DEBUG ormlib] "select * from user where id > 0 limit 2"
[2023-09-08T13:33:22Z DEBUG test::tests] Users = [User { id: 1, name: Some("Mike"), age: 30 }, User { id: 2, name: Some("Mary"), age: 30 }]
[2023-09-08T13:33:22Z DEBUG ormlib] "select * from user where name like \"M%\""
[2023-09-08T13:33:22Z DEBUG test::tests] User = id: 1, name: Some("Mike")
[2023-09-08T13:33:22Z DEBUG test::tests] User = id: 2, name: Some("Mary")
[2023-09-08T13:33:22Z DEBUG ormlib] "update user set age = 100"
[2023-09-08T13:33:22Z DEBUG test::tests] updated_rows: 2
[2023-09-08T13:33:22Z DEBUG ormlib] "delete from user where id = 1"
[2023-09-08T13:33:22Z DEBUG test::tests] updated_rows: 1
```
