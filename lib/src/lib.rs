//! # Ormlib SDK
//! This is the Rust SDK for ORMlib.
//!
//! ## Usage
//!
//! ```toml
//! [dependencies]
//! ormlib = "0.3.1"
//! ormlib_derive = "0.3.1"
//! ```
//!
//! ```rust
//! ```


mod serializer_error;
mod serializer_types;
mod serializer_values;
mod serializer_key_values;
mod deserializer_key_values;

#[cfg(feature = "sqlite")]
pub mod sqlite;

use std::collections::HashMap;
use rusqlite::{Result};

use std::fmt::Debug;
use std::str::FromStr;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ORMError {
    #[error("std::io::Error")]
    StdError(#[from] std::io::Error),
    #[error("rusqlite::Error")]
    RusqliteError(#[from] rusqlite::Error),
    #[error("unknown error")]
    Unknown,
    #[error("Error in object insertion")]
    InsertError,
    #[error("No connection")]
    NoConnection,
}

pub trait TableSerialize {
    fn name(&self) -> String{
        "Test".to_string()
    }
    fn get_id(&self) -> String {
        "0".to_string()
    }
}
pub trait TableDeserialize {
    fn same_name() -> String{
        "Test".to_string()
    }
    fn fields() -> Vec<String>{
        Vec::new()
    }
}



#[derive(Debug)]
pub struct Row {
    pub columns: HashMap<i32,Option<String>>,
}
impl Row {
    pub fn new() -> Self {
        let columns = HashMap::new();
        Row {
            columns
        }
    }
    pub fn get<Z: FromStr>(&self, index: i32) -> Option<Z>
    {
        let value = self.columns.get(&index);
        match value {
            Some(v_opt) => {
                match v_opt {
                    None => {
                        None
                    }
                    Some(v) => {
                        let r = Z::from_str(v.as_str());
                        match r {
                            Ok(res) => {
                                Some(res)
                            }
                            Err(_) => {
                                None
                            }
                        }
                    }
                }

            }
            None => {
                None
            }
        }
    }

    pub fn set<T: ToString>(&mut self, index: i32, value: Option<T>) {
        let value = match value {
            Some(v) => {
                Some(v.to_string())
            }
            None => {
                None
            }
        };
        self.columns.insert(index, value);
    }
}


#[async_trait]
pub trait ORMTrait<O:ORMTrait<O>> {
    // fn connect(url: String) -> Result<Arc<dyn ORMTrait>, ORMError>
    // where Arc<dyn ORMTrait>: Send + Sync + 'static;
    fn add<T>(&self, data: T) -> QueryBuilder<T, T, O>
        where T: for<'a> Deserialize<'a> + TableDeserialize + TableSerialize + Serialize + Debug + 'static;

    async fn last_insert_rowid(&self)  -> Result<i64, ORMError>;


    async fn close(&self)  -> Result<(), ORMError>;

    fn find_one<T: TableDeserialize>(&self, id: u64) -> QueryBuilder<Option<T>, T, O>
    where T: TableDeserialize + TableSerialize + for<'a> Deserialize<'a> + 'static;
    fn find_many<T>(&self, query_where: &str) -> QueryBuilder<Vec<T>, T, O>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static;

    fn find_all<T>(&self) -> QueryBuilder<Vec<T>, T, O>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static;

    fn modify<T>(&self, data: T) -> QueryBuilder<usize, (), O>
        where T: TableDeserialize + TableSerialize + Serialize + 'static;

    fn remove<T>(&self, data: T) -> QueryBuilder<usize, (), O>
        where T: TableDeserialize + TableSerialize + Serialize + 'static;
    fn query<T>(&self, query: &str) -> QueryBuilder<Vec<T>, T, O>;

    fn query_update(&self, query: &str) -> QueryBuilder<usize, (), O>;

    fn protect(&self, value: &str) -> String;
    fn escape(str: &str) -> String;
    fn escape_json(input: &str) -> String;


    async fn init(&self, script: &str) -> Result<(), ORMError>;
}

pub struct QueryBuilder<'a, R, E, O: ORMTrait<O>> {
    query: String,
    entity:  std::marker::PhantomData<E>,
    orm: &'a O,
    result: std::marker::PhantomData<R>,
}




#[cfg(test)]
mod tests {
    use crate::ORMError;

    #[tokio::test]
    async fn test() -> Result<(), ORMError> {
        Ok(())
    }
}