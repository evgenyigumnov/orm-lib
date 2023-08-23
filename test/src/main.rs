use ormlib::ORMError;



#[tokio::main]
async fn main() -> Result<(), ORMError> {
    Ok(())
}


#[cfg(test)]
mod tests {

    use orm_derive::Table;
    use ormlib::Table;
    use ormlib::ORMError;





    #[derive(Table, Debug)]
    pub struct TestA;
    #[derive(Table, Debug)]
    #[table(name = "B")]
    pub struct TestB;

    #[tokio::test]
    async fn test_derive() -> Result<(), ORMError> {

        let t = TestA {};
        assert_eq!(t.name(), "TestA");
        let t = TestB {};
        assert_eq!(t.name(), "B");

        Ok(())
    }

    #[derive(Debug, Clone)]
    pub struct User {
        pub id: i32,
        pub name: String,
    }


    use ormlib::{ORM, Row};

    #[tokio::test]
    async fn test() -> Result<(), ORMError> {
        let conn = ORM::connect("file.db".to_string());
        let init_script = "create_table_1.sql".to_string();
        conn.init(init_script).await?;


        let query = format!("SELECT * FROM User WHERE name like {}", conn.protect("John".to_string()));
        let result_set: Vec<Row> = conn.query(query).run().await?;
        for row in result_set {
            let id = row.get::<i32>("id");
            let name = row.get::<String>("name");
        }
        let query = "delete from User".to_string();
        let updated_rows: i32 = conn.query_update(query).run().await?;



        let user = User {
            id: 0,
            name: "John".to_string(),
        };

        conn.insert(user.clone()).run().await?;
        let user_opt: Option<User> = conn.findOne("id = 1".to_string()).run().await?;
        let user_opt: Vec<User> = conn.findMany("id > 0".to_string()).run().await?;
        let user_opt: Vec<User> = conn.findAll().limit(10).run().await?;
        let updated_rows: i32 = conn.update(user.clone(), "id = 1".to_string()).run().await?;


        Ok(())
    }
}

