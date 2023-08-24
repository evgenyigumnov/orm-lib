use ormlib::ORMError;



#[tokio::main]
async fn main() -> Result<(), ORMError> {
    Ok(())
}


#[cfg(test)]
mod tests {
    use serde_derive::{Deserialize, Serialize};
    use orm_derive::TableDeserialize;
    use ormlib::{RowTrait, TableDeserialize};
    use orm_derive::TableSerialize;
    use ormlib::TableSerialize;
    use ormlib::ORMError;

    #[derive(TableSerialize, TableDeserialize, Debug)]
    #[table(name = "B")]
    pub struct TestB {
        pub id: i32,
        pub id_id: i32,
    }

    #[tokio::test]
    async fn test_derive() -> Result<(), ORMError> {

        let t = TestB { id: 0, id_id: 0 };
        assert_eq!(t.name(), "B");
        assert_eq!(TestB::same_name(), "B");
        let r = format!("{:?}", TestB::fields());
        assert_eq!(r, "[\"id\", \"id_id\"]");
        Ok(())
    }

    use ormlib::{ORM, Row};

    #[tokio::test]
    async fn test() -> Result<(), ORMError> {

        #[derive(TableDeserialize, TableSerialize, Serialize, Deserialize, Debug, Clone)]
        #[table(name = "user")]
        pub struct User {
            pub id: i32,
            pub name: Option<String>,
            pub age: i32,
        }

        // if file file.db exists, it will be deleted
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
}

