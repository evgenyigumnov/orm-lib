# orm-lib


## Usage

Cargo.toml

```toml
[dependencies]
ormlib = "0.1.0"
```


```rust
   #[tokio::test]
    async fn test() -> Result<(), ORMError> {

        #[derive(TableDeserialize, TableSerialize, Serialize, Deserialize, Debug, Clone)]
        #[table(name = "user")]
        pub struct User {
            pub id: i32,
            pub name: Option<String>,
            pub age: i32,
        }

        let file = std::path::Path::new("file.db");
        if file.exists() {
            std::fs::remove_file(file)?;
        }

        let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("debug")).try_init();
        let user = User {
            id: 0,
            name: Some("John".to_string()),
            age: 30,
        };

        let conn = ORM::connect("file.db".to_string())?;
        let init_script = "create_table_1.sql".to_string();
        conn.init(init_script).await?;
        let insert_id: i64 = conn.insert(user.clone()).run().await?;
        log::debug!("insert_id: {}", insert_id);
        let updated_rows: usize = conn.query_update("insert into user (id, age) values (2, 33)".to_string()).exec().await?;

        let query = format!("select * from user where name like {}", conn.protect("%oh%".to_string()));
        let result_set: Vec<Row> = conn.query(query).exec().await?;
        for row in result_set {
            let id: i32 = row.get(0).unwrap();
            let name: Option<String> = row.get(1);
            log::debug!("id: {}, name: {:?}", id, name);
        }


        let user_opt: Option<User> = conn.findOne(format!("id = {insert_id}")).run().await?;
        log::debug!("{:?}", user_opt);
        let user = User {
            id: 0,
            name: None,
            age: 40,
        };
        let insert_id:i64 = conn.insert(user.clone()).run().await?;
        log::debug!("insert_id: {}", insert_id);


        let user_vec: Vec<User> = conn.findMany("id > 0".to_string()).limit(2).run().await?;
        log::debug!("{:?}", user_vec);
        let user_vec: Vec<User> = conn.findAll().run().await?;
        log::debug!("{:?}", user_vec);
        let updated_rows: usize = conn.update(user.clone(), "id = 1".to_string()).run().await?;
        let user_vec: Vec<User> = conn.findAll().run().await?;
        log::debug!("{:?}", user_vec);
        let query = "delete from user".to_string();
        let updated_rows = conn.query_update(query).exec().await?;
        log::debug!("updated_rows: {}", updated_rows);
        Ok(())
    }
```


Example output:

```text 
[2023-08-24T07:56:47Z DEBUG ormlib] CREATE TABLE user (
                            id    INTEGER PRIMARY KEY AUTOINCREMENT,
                            name  TEXT,
                            age    INTEGER
    )
[2023-08-24T07:56:47Z DEBUG ormlib] insert into user (name,age) values ("John",30)
[2023-08-24T07:56:47Z DEBUG test::tests] insert_id: 1
[2023-08-24T07:56:47Z DEBUG ormlib] insert into user (id, age) values (2, 33)
[2023-08-24T07:56:47Z DEBUG ormlib] select * from user where name like "%oh%"
[2023-08-24T07:56:47Z DEBUG test::tests] id: 1, name: Some("John")
[2023-08-24T07:56:47Z DEBUG ormlib] select * from user where id = 1
[2023-08-24T07:56:47Z DEBUG test::tests] Some(User { id: 1, name: Some("John"), age: 30 })
[2023-08-24T07:56:47Z DEBUG ormlib] insert into user (name,age) values (null,40)
[2023-08-24T07:56:47Z DEBUG test::tests] insert_id: 3
[2023-08-24T07:56:47Z DEBUG ormlib] select * from user where id > 0 LIMIT 2
[2023-08-24T07:56:47Z DEBUG test::tests] [User { id: 1, name: Some("John"), age: 30 }, User { id: 2, name: None, age: 33 }]
[2023-08-24T07:56:47Z DEBUG ormlib] select * from user
[2023-08-24T07:56:47Z DEBUG test::tests] [User { id: 1, name: Some("John"), age: 30 }, User { id: 2, name: None, age: 33 }, User { id: 3, name: None, age: 40 }]
[2023-08-24T07:56:47Z DEBUG ormlib] update user set name = null,age = 40 where id = 1
[2023-08-24T07:56:47Z DEBUG ormlib] select * from user
[2023-08-24T07:56:47Z DEBUG test::tests] [User { id: 1, name: None, age: 40 }, User { id: 2, name: None, age: 33 }, User { id: 3, name: None, age: 40 }]
[2023-08-24T07:56:47Z DEBUG ormlib] delete from user
[2023-08-24T07:56:47Z DEBUG test::tests] updated_rows: 3
```
