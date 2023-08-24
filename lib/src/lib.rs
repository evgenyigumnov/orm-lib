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
mod deserializer_key_values;

use std::collections::HashMap;
use rusqlite::{Connection, Result};

use std::fmt::Debug;
use std::str::FromStr;
use std::sync::Arc;
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
}

pub trait TableSerialize {
    fn name(&self) -> String{
        "Test".to_string()
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
pub struct ORM {
    conn: Connection,
}

#[derive(Debug)]
pub struct Row {
    pub columns: HashMap<String,Option<String>>,
}
pub trait RowTrait {
    fn get<T: FromStr>(&self, name: &str) -> Option<T>;
    fn set<T: ToString>(&mut self, name: String, value: Option<T>);
}
impl Row {
    pub fn new() -> Self {
        let columns = HashMap::new();
        Row {
            columns
        }
    }
}

impl RowTrait for Row {
    fn get<Z: FromStr>(&self, name: &str) -> Option<Z>
    {
        let value = self.columns.get(name);
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

    fn set<T: ToString>(&mut self, name: String, value: Option<T>) {
        let value = match value {
            Some(v) => {
                Some(v.to_string())
            }
            None => {
                None
            }
        };
        self.columns.insert(name.to_string(), value);
    }
}


impl ORM {
    pub fn connect(url: String) -> Result<Arc<ORM>, ORMError> {
        let conn = Connection::open(url)?;
        Ok(Arc::new(ORM { conn }))
    }
    pub fn insert<T>(&self, data: T) -> QueryBuilder<usize, T>
    where T: TableDeserialize + TableSerialize + Serialize + 'static
    {
        let table_name = data.name();
        let types = serializer_types::to_string(&data).unwrap();
        let values = serializer_values::to_string(&data).unwrap();
        let query: String = format!("insert into {table_name} {types} values {values}");
        let qb = QueryBuilder::<usize,T> {
            query: query,
            entity: Some(data),
            orm: self,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn last_insert_rowid(&self) -> i64 {
        self.conn.last_insert_rowid()
    }

    pub fn findOne<T: TableDeserialize>(&self, query_where: String) -> QueryBuilder<Option<T>, T>
    where T: TableDeserialize + TableSerialize + for<'a> Deserialize<'a> + 'static
    {
        let table_name = T::same_name();

        let query: String = format!("select * from {table_name} where {query_where}");
        log::debug!("{}", query);

        let qb = QueryBuilder::<Option<T>, T> {
            query,
            entity: None,
            orm: self,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn findMany<T>(&self, query: String) -> QueryBuilder<Vec<T>, T> {
        let qb = QueryBuilder::<Vec<T>, T> {
            query,
            entity: None,
            orm: self,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn findAll<T>(&self) -> QueryBuilder<Vec<T>, ()> {
        let qb = QueryBuilder::<Vec<T>, ()> {
            query: "SELECT * FROM ".to_string(),
            entity: None,
            orm: self,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn update<T>(&self, data: T, query: String) -> QueryBuilder<i32, T> {
        let qb = QueryBuilder::<i32, T> {
            query,
            entity: Some(data),
            orm: self,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn query<T>(&self, query: String) -> QueryBuilder<Vec<T>, T> {
           let qb = QueryBuilder::<Vec<T>, T> {
            query,
            entity: None,
            orm: self,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn query_update(&self, query: String) -> QueryBuilder<usize, ()> {
        let qb = QueryBuilder::<usize, ()> {
            query,
            entity: None,
            orm: self,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn protect(&self, value: String) -> String {
        let protected: String = format!("\"{}\"", ORM::escape(value));
        protected

    }
    pub fn escape(str:String) -> String {
        str
    }

    pub async fn init(&self, script: String) -> Result<(), ORMError>  {
        let query = std::fs::read_to_string(script)?;
        let updated_rows: usize = self.query_update(query).run().await?;

        Ok(())
    }
}

pub struct QueryBuilder<'a, T, V> {
    query: String,
    entity: Option<V>,
    orm: &'a ORM,
    phantom: std::marker::PhantomData<T>,
}

impl<T> QueryBuilder<'_, usize,T> {
    pub async fn run(&self) -> Result<usize, ORMError> {
        log::debug!("{}", self.query);
        let r = self.orm.conn.execute(self.query.as_str(),(),)?;
        Ok(r)
    }
}

impl<T> QueryBuilder<'_, Option<T>,T>
where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static
{
    pub async fn run(&self) -> Result<Option<T>, ORMError> {

        let rows  = self.orm.query(self.query.clone()).run().await?;
        let columns: Vec<String> =T::fields();
        if rows.len() == 0 {
            return Ok(None);
        } else {
            let mut column_str: Vec<String> = Vec::new();
            for row in rows {
                let mut i = 0;
                for column in columns.iter() {
                    let value_opt:Option<String> = row.get(i.to_string().as_str());
                    let value = match value_opt {
                        Some(v) => {
                            format!("\"{}\"", v.to_string())
                        }
                        None => {
                            "null".to_string()
                        }
                    };
                    column_str.push(format!("\"{}\":{}", column, value));
                    i = i + 1;
                }
            }
            println!("{:?}", column_str);
            let user_str = r#"{"id":"1","name":"John"}"#.to_string();
            let user: T = deserializer_key_values::from_str(&user_str).unwrap();

            log::debug!("{:?}", user);

            Ok(Some(user))

        }

    }
}

impl<R> QueryBuilder<'_, Vec<Row>,R> {
    pub async fn run(&self) -> Result<Vec<Row>, ORMError>
    {
        log::debug!("{}", self.query);
        let mut stmt = self.orm.conn.prepare( self.query.as_str())?;
        let mut result: Vec<Row> = Vec::new();
        let person_iter = stmt.query_map([], |row| {
            let mut i = 0;
            let mut r: Row = Row::new();
            loop {
                let res: rusqlite::Result<i32>= row.get(i);

                match  res{
                    Ok(v) => {
                        r.set(i.to_string(), Some(v));

                    },
                    Err(e) => {
                        if e ==  rusqlite::Error::InvalidColumnIndex(i) {
                            break;
                        }
                    }
                }

                let res: rusqlite::Result<String>= row.get(i);
                match  res{

                    Ok(v) => {
                        r.set(i.to_string(), Some(v));
                    }
                    Err(e) => {
                    }
                }

                i = i + 1;
            }

            result.push(r);
            Ok(())
        })?;
        for x in person_iter {
        }
        // log::debug!("{:?}", result);

        Ok(result)
    }

    pub fn limit(&self, limit: i32) -> QueryBuilder<Vec<Row>, ()> {

        let qb =  QueryBuilder::<Vec<Row>, ()> {
            query: format!("{} LIMIT {}", self.query, limit),
            entity: None,
            orm: self.orm,
            phantom: std::marker::PhantomData,
        };
        qb
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