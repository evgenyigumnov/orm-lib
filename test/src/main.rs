use ormlib::ORMError;



#[tokio::main]
async fn main() -> Result<(), ORMError> {
    Ok(())
}


#[cfg(test)]
mod tests {
    use serde_derive::{Deserialize, Serialize};
    use orm_derive::TableDeserialize;
    use ormlib::TableDeserialize;
    use orm_derive::TableSerialize;
    use ormlib::TableSerialize;
    use ormlib::ORMError;

    #[derive(TableSerialize, TableDeserialize, Debug)]
    #[table(name = "B")]
    pub struct TestB {
        pub id: i32,
    }

    #[tokio::test]
    async fn test_derive() -> Result<(), ORMError> {

        let t = TestB { id: 0 };
        assert_eq!(t.name(), "B");
        assert_eq!(TestB::same_name(), "B");

        Ok(())
    }

    use ormlib::{ORM, Row};

    #[tokio::test]
    async fn test() -> Result<(), ORMError> {

        #[derive(TableDeserialize, TableSerialize, Serialize, Deserialize, Debug, Clone)]
        #[table(name = "user")]
        pub struct User {
            pub id: i32,
            pub name: String,
        }



        let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("debug")).try_init();

        let conn = ORM::connect("file.db".to_string());
        let init_script = "create_table_1.sql".to_string();
        conn.init(init_script).await?;




        let user = User {
            id: 0,
            name: "John".to_string(),
        };

        let insert_id = conn.insert(user.clone()).run().await?;
        let user_opt: Option<User> = conn.findOne(format!("id = {insert_id}")).run().await?;
        let query = format!("select * from user where name like {}", conn.protect("%oh%".to_string()));
        let result_set: Vec<Row> = conn.query(query).run().await?;
        for row in result_set {
            let id = row.get::<i32>("id");
            let name = row.get::<String>("name");
        }
        // let user_opt: Vec<User> = conn.findMany("id > 0".to_string()).run().await?;
        // let user_opt: Vec<User> = conn.findAll().limit(10).run().await?;
        // let updated_rows: i32 = conn.update(user.clone(), "id = 1".to_string()).run().await?;
        //

        // let query = "delete from User".to_string();
        // let updated_rows: i32 = conn.query_update(query).run().await?;


        Ok(())
    }
}

