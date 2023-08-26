# Ormlib

Indeed, an ORM library, not a framework, written in Rust

## Features

The main idea that I put into my ORM library is a minimum of stupid code and easy use of the library. I wanted users not to have to write long chains of function calls to construct a simple SQL query.


## Usage

Cargo.toml

```toml
[dependencies]
ormlib = "0.2.3"
ormlib_derive = "0.2.3"
```


```rust
#[tokio::test]
async fn test() -> Result<(), ORMError> {

    let file = std::path::Path::new("file.db");
    if file.exists() {
        std::fs::remove_file(file)?;
    }

    let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("debug")).try_init();

    let conn = ORM::connect("file.db".to_string())?;
    let init_script = "create_table_1.sql";
    conn.init(init_script).await?;

    #[derive(TableDeserialize, TableSerialize, Serialize, Deserialize, Debug, Clone)]
    #[table(name = "user")]
    pub struct User {
        pub id: i32,
        pub name: Option<String>,
        pub age: i32,
    }

    let mut user = User {
        id: 0,
        name: Some("John".to_string()),
        age: 30,
    };

    let mut user_from_db: User = conn.insert(user.clone()).apply().await?;

    user.name = Some("Mary".to_string());
    let  _: User = conn.insert(user.clone()).apply().await?;

    let query_where = format!("id = {}", user_from_db.id);
    let user_opt: Option<User> = conn.find_one(query_where.as_str()).run().await?;
    log::debug!("User = {:?}", user_opt);

    let user_all: Vec<User> = conn.find_all().run().await?;
    log::debug!("Users = {:?}", user_all);

    user_from_db.name = Some("Mike".to_string());
    let _updated_rows: usize = conn.update(user_from_db, query_where.as_str()).run().await?;


    let user_many: Vec<User> = conn.find_many("id > 0").limit(2).run().await?;
    log::debug!("Users = {:?}", user_many);

    let query = format!("select * from user where name like {}", conn.protect("%oh%"));
    let result_set: Vec<Row> = conn.query(query.as_str()).exec().await?;
    for row in result_set {
        let id: i32 = row.get(0).unwrap();
        let name: Option<String> = row.get(1);
        log::debug!("User = id: {}, name: {:?}", id, name);
    }

    let updated_rows = conn.query_update("delete from user").exec().await?;
    log::debug!("updated_rows: {}", updated_rows);
    Ok(())
}
```


Example output:

```text 
[2023-08-25T02:40:22Z DEBUG ormlib] CREATE TABLE user (
                            id    INTEGER PRIMARY KEY AUTOINCREMENT,
                            name  TEXT,
                            age    INTEGER
    )
[2023-08-25T02:40:22Z DEBUG ormlib] insert into user (name,age) values ("John",30)
[2023-08-25T02:40:22Z DEBUG ormlib] select * from user where rowid = 1
[2023-08-25T02:40:22Z DEBUG ormlib] insert into user (name,age) values ("Mary",30)
[2023-08-25T02:40:22Z DEBUG ormlib] select * from user where rowid = 2
[2023-08-25T02:40:22Z DEBUG ormlib] select * from user where id = 1
[2023-08-25T02:40:22Z DEBUG test::tests] User = Some(User { id: 1, name: Some("John"), age: 30 })
[2023-08-25T02:40:22Z DEBUG ormlib] select * from user
[2023-08-25T02:40:22Z DEBUG test::tests] Users = [User { id: 1, name: Some("John"), age: 30 }, User { id: 2, name: Some("Mary"), age: 30 }]
[2023-08-25T02:40:22Z DEBUG ormlib] update user set name = "Mike",age = 30 where id = 1
[2023-08-25T02:40:22Z DEBUG ormlib] select * from user where id > 0 LIMIT 2
[2023-08-25T02:40:22Z DEBUG test::tests] Users = [User { id: 1, name: Some("Mike"), age: 30 }, User { id: 2, name: Some("Mary"), age: 30 }]
[2023-08-25T02:40:22Z DEBUG ormlib] select * from user where name like "%oh%"
[2023-08-25T02:40:22Z DEBUG ormlib] delete from user
[2023-08-25T02:40:22Z DEBUG test::tests] updated_rows: 2
```
