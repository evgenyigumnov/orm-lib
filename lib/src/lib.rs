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
    pub fn insert<T>(&self, data: T) -> QueryBuilder<i32, T> {
        let qb = QueryBuilder::<i32, T> {
            query: "INSERT INTO ".to_string(),
            entity: Some(data),
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn findOne<T>(&self, query: String) -> QueryBuilder<Option<T>, T> {
        let qb = QueryBuilder::<Option<T>, T> {
            query,
            entity: None,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn findMany<T>(&self, query: String) -> QueryBuilder<Vec<T>, T> {
        let qb = QueryBuilder::<Vec<T>, T> {
            query,
            entity: None,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn findAll<T>(&self) -> QueryBuilder<Vec<T>, ()> {
        let qb = QueryBuilder::<Vec<T>, ()> {
            query: "SELECT * FROM ".to_string(),
            entity: None,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn update<T>(&self, data: T, query: String) -> QueryBuilder<i32, T> {
        let qb = QueryBuilder::<i32, T> {
            query,
            entity: Some(data),
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn query<T>(&self, query: String) -> QueryBuilder<Vec<T>, T> {
           let qb = QueryBuilder::<Vec<T>, T> {
            query,
            entity: None,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn query_update(&self, query: String) -> QueryBuilder<i32, ()> {
        let qb = QueryBuilder::<i32, ()> {
            query,
            entity: None,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn protect(&self, value: String) -> String {
        ORM::escape(value)
    }
    pub fn escape(str:String) -> String {
        str
    }

    pub async fn init(&self, script: String) -> Result<(), ORMError>  {
        Ok(())
    }
}

pub struct QueryBuilder<T,V> {
    query: String,
    entity: Option<V>,
    phantom: std::marker::PhantomData<T>,
}

impl QueryBuilder<i32, Box<dyn Table>> {
    pub async fn run(&self) -> Result<i32, ORMError> {
        let r:i32  = 1;
        Ok(r)
    }
}

impl<Z> QueryBuilder<Option<Z>, Box<dyn Table>> {
    pub async fn run(&self) -> Result<Option<Z>, ORMError> {
        Ok(None)
    }
}

impl<Z> QueryBuilder<Vec<Z>, Box<dyn Table>> {
    pub async fn run(&self) -> Result<Vec<Z>, ORMError> {
        Ok(Vec::new())
    }

    pub fn limit(&self, limit: i32) -> QueryBuilder<Vec<Z>, ()> {

        let qb =  QueryBuilder::<Vec<Z>, ()> {
            query: format!("{} LIMIT {}", self.query, limit),
            entity: None,
            phantom: std::marker::PhantomData,
        };
        qb
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
    use crate::ORMError;

    #[tokio::test]
    async fn test() -> Result<(), ORMError> {
        Ok(())
    }
}