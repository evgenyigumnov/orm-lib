//! # Actorlib SDK
//! This is the Rust SDK for ORMlib.
//!
//! ## Usage
//! ```rust
//!use ormlib::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), std::io::Error> {
//!
//!    Ok(())
//! }

mod serializer_error;
mod serializer_types;
mod serializer_values;
mod serializer_key_values;
use std::sync::Arc;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ORMError {
    #[error("std::io::Error")]
    StdError(#[from] std::io::Error),
    #[error("unknown error")]
    Unknown,
}

pub trait Table {
    fn name(&self) -> String{
        "Test".to_string()
    }
}

pub struct ORM {}

impl ORM {
    pub fn connect(url: String) -> Arc<ORM> {
        Arc::new(ORM {})
    }
    pub fn insert<T>(&self, data: T) -> QueryBuilder<i32> {
        todo!()
    }

    pub fn findOne<T>(&self, query: String) -> QueryBuilder<Option<T>> {
        todo!()
    }

    pub fn findMany<T>(&self, query: String) -> QueryBuilder<Vec<T>> {
        todo!()
    }

    pub fn findAll<T>(&self) -> QueryBuilder<Vec<T>> {
        todo!()
    }

    pub fn update<T>(&self, data: T, query: String) -> QueryBuilder<i32> {
        todo!()
    }

    pub fn query<T>(&self, query: String) -> QueryBuilder<Vec<T>> {
        todo!()
    }

    pub fn query_update(&self, query: String) -> QueryBuilder<i32> {
        todo!()
    }

    pub fn protect(&self, value: String) -> String {
        ORM::escape(value)
    }
    pub fn escape(str:String) -> String {
        str
    }

    pub async fn init(&self, script: String) -> Result<(), ORMError>  {
        todo!()
    }
}

pub struct QueryBuilder<T> {
    query: String,
    phantom: std::marker::PhantomData<T>,
}

impl QueryBuilder<i32> {
    pub async fn run<i32>(&self) -> Result<i32, ORMError> {
        todo!()
    }
}

impl<Z> QueryBuilder<Option<Z>> {
    pub async fn run(&self) -> Result<Option<Z>, ORMError> {
        todo!()
    }
}

impl<Z> QueryBuilder<Vec<Z>> {
    pub async fn run(&self) -> Result<Vec<Z>, ORMError> {
        todo!()
    }

    pub fn limit(&self, limit: i32) -> QueryBuilder<Vec<Z>> {
        todo!()
    }
}


pub struct Row;

impl Row {
    pub fn get<T>(&self, name: &str) -> T {
        todo!()
    }
}


#[cfg(test)]
mod tests {
    #[derive(Debug, Clone)]
    pub struct User {
        pub id: i32,
        pub name: String,
    }


    use crate::{ORM, ORMError, Row};

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