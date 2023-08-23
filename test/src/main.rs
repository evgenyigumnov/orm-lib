use ormlib::ORMError;



#[tokio::main]
async fn main() -> Result<(), ORMError> {
    Ok(())
}


#[cfg(test)]
mod tests {
    use serde_derive::Serialize;
    use orm_derive::Table;
    use ormlib::Table;
    use ormlib::ORMError;





    #[derive(Table, Debug)]
    #[table(name = "B")]
    pub struct TestB {
        pub id: i32,
    }



    #[tokio::test]
    async fn test_derive() -> Result<(), ORMError> {

        let t = TestB { id: 0 };
        assert_eq!(t.name(), "B");

        Ok(())
    }




    use ormlib::{ORM, Row};

    #[tokio::test]
    async fn test() -> Result<(), ORMError> {

        #[derive(Table, Serialize, Debug, Clone)]
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

        conn.insert(user.clone()).run().await?;
        let user_opt: Option<User> = conn.findOne("id = 1".to_string()).run().await?;
        // let user_opt: Vec<User> = conn.findMany("id > 0".to_string()).run().await?;
        // let user_opt: Vec<User> = conn.findAll().limit(10).run().await?;
        // let updated_rows: i32 = conn.update(user.clone(), "id = 1".to_string()).run().await?;
        //
        // let query = format!("SELECT * FROM User WHERE name like {}", conn.protect("John".to_string()));
        // let result_set: Vec<Row> = conn.query(query).run().await?;
        // for row in result_set {
        //     let id = row.get::<i32>("id");
        //     let name = row.get::<String>("name");
        // }
        // let query = "delete from User".to_string();
        // let updated_rows: i32 = conn.query_update(query).run().await?;


        Ok(())
    }
}

