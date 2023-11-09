use ormlib::ORMError;



#[tokio::main]
async fn main() -> Result<(), ORMError> {
    Ok(())
}


#[cfg(test)]
mod tests {
    use serde_derive::{Deserialize, Serialize};
    use ormlib_derive::TableDeserialize;
    use ormlib::{ORMTrait, TableDeserialize};
    use ormlib_derive::TableSerialize;
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

    use ormlib::{Row};
    use ormlib::sqlite::ORM;


    // ANCHOR: readme_example
    #[tokio::test]
    async fn test() -> Result<(), ORMError> {

        let file = std::path::Path::new("file1.db");
        if file.exists() {
            std::fs::remove_file(file)?;
        }

        let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("debug")).try_init();

        let conn = ORM::connect("file1.db".to_string())?;
        let init_script = "create_table_sqlite.sql";
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

        let mut user_from_db: User = conn.add(user.clone()).apply().await?;

        user.name = Some("Mary".to_string());
        let  _: User = conn.add(user.clone()).apply().await?;

        let user_opt: Option<User> = conn.find_one(user_from_db.id as u64).run().await?;
        log::debug!("User = {:?}", user_opt);

        let user_all: Vec<User> = conn.find_all().run().await?;
        log::debug!("Users = {:?}", user_all);

        user_from_db.name = Some("Mike".to_string());
        let _updated_rows: usize = conn.modify(user_from_db.clone()).run().await?;


        let user_many: Vec<User> = conn.find_many("id > 0").limit(2).run().await?;
        log::debug!("Users = {:?}", user_many);

        let query = format!("select * from user where name like {}", conn.protect("M%"));
        let result_set: Vec<Row> = conn.query(query.as_str()).exec().await?;
        for row in result_set {
            let id: i32 = row.get(0).unwrap();
            let name: Option<String> = row.get(1);
            log::debug!("User = id: {}, name: {:?}", id, name);
        }

        let updated_rows = conn.query_update("update user set age = 100").exec().await?;
        log::debug!("updated_rows: {}", updated_rows);
        let updated_rows: usize = conn.remove(user_from_db.clone()).run().await?;
        log::debug!("updated_rows: {}", updated_rows);
        conn.close().await?;

        Ok(())
    }
    // ANCHOR_END: readme_example

    #[tokio::test]
    async fn test_dirty() -> Result<(), ORMError> {

        #[derive(TableDeserialize, TableSerialize, Serialize, Deserialize, Debug, Clone)]
        #[table(name = "user")]
        pub struct User {
            pub id: i32,
            pub name: Option<String>,
            pub age: i32,
        }

        let file = std::path::Path::new("file2.db");
        if file.exists() {
            std::fs::remove_file(file)?;
        }

        let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("debug")).try_init();
        let user = User {
            id: 0,
            name: Some("John".to_string()),
            age: 30,
        };

        let conn = ORM::connect("file2.db".to_string())?;
        let init_script = "create_table_sqlite.sql";
        conn.init(init_script).await?;
        let user_from_db: User = conn.add(user.clone()).apply().await?;
        log::debug!("insert_id: {}", user_from_db.id);
        let _updated_rows: usize = conn.query_update("insert into user (id, age) values (2, 33)").exec().await?;

        let query = format!("select * from user where name like {}", conn.protect("%oh%"));
        let result_set: Vec<Row> = conn.query(query.as_str()).exec().await?;
        for row in result_set {
            let id: i32 = row.get(0).unwrap();
            let name: Option<String> = row.get(1);
            log::debug!("id: {}, name: {:?}", id, name);
        }


        let inseret_id = user_from_db.id;
        let user_opt: Option<User> = conn.find_one(inseret_id as u64).run().await?;
        log::debug!("{:?}", user_opt);
        let input = "Hello c:\\temp 'world' \r \t and \"universe\"";

        let user = User {
            id: 0,
            name: Some(input.to_string()),
            age: 40,
        };
        let new_user = conn.add(user.clone()).apply().await?;
        log::debug!("insert_id: {}", new_user.id);
        let user_opt: Option<User> = conn.find_one(3).run().await?;
        assert_eq!(input, user_opt.unwrap().name.unwrap());

        let user_vec: Vec<User> = conn.find_many("id > 0").limit(2).run().await?;
        log::debug!("{:?}", user_vec);
        let user_vec: Vec<User> = conn.find_all().run().await?;
        log::debug!("{:?}", user_vec);
        let _updated_rows: usize = conn.modify(user.clone()).run().await?;
        let user_vec: Vec<User> = conn.find_all().run().await?;
        log::debug!("{:?}", user_vec);
        let updated_rows = conn.query_update("delete from user").exec().await?;
        log::debug!("updated_rows: {}", updated_rows);
        conn.close().await?;
        Ok(())
    }



    #[tokio::test]
    async fn test_async() -> Result<(), ORMError> {
        let file = std::path::Path::new("file3.db");
        if file.exists() {
            std::fs::remove_file(file)?;
        }
        let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("debug")).try_init();

        let conn = ORM::connect("file3.db".to_string())?;

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let r = runtime.spawn(async move {
            let init_script = "create_table_sqlite.sql";
            conn.init(init_script).await.unwrap();
            conn.close().await.unwrap();
        });
        r.await.unwrap();
        std::mem::forget(runtime);

        Ok(())
    }


    #[tokio::test]
    async fn test_remove() -> Result<(), ORMError> {

        #[derive(TableDeserialize, TableSerialize, Serialize, Deserialize, Debug, Clone,PartialEq)]
        #[table(name = "user")]
        pub struct User {
            pub id: i32,
            pub name: Option<String>,
            pub age: i32,
        }

        let file = std::path::Path::new("file4.db");
        if file.exists() {
            std::fs::remove_file(file)?;
        }

        let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("debug")).try_init();
        let user = User {
            id: 0,
            name: Some("John".to_string()),
            age: 30,
        };

        let conn = ORM::connect("file4.db".to_string())?;
        let init_script = "create_table_sqlite.sql";
        conn.init(init_script).await?;
        let user_from_db: User = conn.add(user.clone()).apply().await?;
        log::debug!("insert_id: {}", user_from_db.id);
        let _updated_rows: usize = conn.remove(user_from_db.clone()).run().await?;
        let user_opt: Option<User> = conn.find_one(user_from_db.id as u64).run().await?;
        assert_eq!(None, user_opt);
        conn.close().await?;
        Ok(())
    }



    #[tokio::test]
    async fn test_ver() -> Result<(), ORMError> {
        let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("debug")).try_init();
        test_ver_impl().await?;
        test_ver_impl().await?;
        Ok(())
    }
    async fn test_ver_impl() -> Result<(), ORMError> {

        let conn = ORM::connect("file5.db".to_string())?;

        let change_1 = "CREATE TABLE user (id INTEGER PRIMARY KEY AUTOINCREMENT, name  TEXT,age INTEGER)";
        let change_2 = "ALTER TABLE user DROP COLUMN age";
        conn.change(change_1).await.unwrap();
        conn.change(change_2).await.unwrap();
        conn.close().await.unwrap();

        Ok(())
    }


    #[tokio::test]
    async fn test_remove_mysql() -> Result<(), ORMError> {

        #[derive(TableDeserialize, TableSerialize, Serialize, Deserialize, Debug, Clone,PartialEq)]
        #[table(name = "user")]
        pub struct User {
            pub id: i32,
            pub name: Option<String>,
            pub age: i32,
        }

        let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("debug")).try_init();
        let user = User {
            id: 0,
            name: Some("John".to_string()),
            age: 30,
        };

        let conn = ormlib::mysql::ORM::connect("mysql://root:root@192.168.145.128:3306/tests".to_string()).await?;
        let init_script = "create_table_mysql.sql";
        let _ = conn.init(init_script).await;
        let user_from_db: User = conn.add(user.clone()).apply().await?;
        log::debug!("insert_id: {}", user_from_db.id);
        let _updated_rows: usize = conn.remove(user_from_db.clone()).run().await?;
        let user_opt: Option<User> = conn.find_one(user_from_db.id as u64).run().await?;
        assert_eq!(None, user_opt);
        let _ = conn.query_update("drop table user").exec().await?;
        conn.close().await?;

        Ok(())
    }



    #[tokio::test]
    async fn test_mysql() -> Result<(), ORMError> {

        let file = std::path::Path::new("file1.db");
        if file.exists() {
            std::fs::remove_file(file)?;
        }

        let _ = env_logger::Builder::from_env(env_logger::Env::new().default_filter_or("debug")).try_init();

        let conn = ormlib::mysql::ORM::connect("mysql://root:root@192.168.145.128:3306/tests".to_string()).await?;
        let init_script = "create_table_mysql.sql";
        let _ = conn.init(init_script).await;

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

        let mut user_from_db: User = conn.add(user.clone()).apply().await?;

        user.name = Some("Mary".to_string());
        let  _: User = conn.add(user.clone()).apply().await?;

        let user_opt: Option<User> = conn.find_one(user_from_db.id as u64).run().await?;
        log::debug!("User = {:?}", user_opt);

        let user_all: Vec<User> = conn.find_all().run().await?;
        log::debug!("Users = {:?}", user_all);

        user_from_db.name = Some("Mike".to_string());
        let _updated_rows: usize = conn.modify(user_from_db.clone()).run().await?;


        let user_many: Vec<User> = conn.find_many("id > 0").limit(2).run().await?;
        log::debug!("Users = {:?}", user_many);

        let query = format!("select * from user where name like {}", conn.protect("M%"));
        let result_set: Vec<Row> = conn.query(query.as_str()).exec().await?;
        for row in result_set {
            let id: i32 = row.get(0).unwrap();
            let name: Option<String> = row.get(1);
            log::debug!("User = id: {}, name: {:?}", id, name);
        }

        let updated_rows = conn.query_update("update user set age = 100").exec().await?;
        log::debug!("updated_rows: {}", updated_rows);
        let updated_rows: usize = conn.remove(user_from_db.clone()).run().await?;
        log::debug!("updated_rows: {}", updated_rows);
        let _ = conn.query_update("drop table user").exec().await?;

        conn.close().await?;

        Ok(())
    }

}

