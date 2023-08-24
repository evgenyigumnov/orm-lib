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
use crate::serializer_types::to_string;

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
    pub columns: HashMap<i32,Option<String>>,
}
pub trait RowTrait {
    fn get<T: FromStr>(&self, index: i32) -> Option<T>;
    fn set<T: ToString>(&mut self, index: i32, value: Option<T>);
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
    fn get<Z: FromStr>(&self, index: i32) -> Option<Z>
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

    fn set<T: ToString>(&mut self, index: i32, value: Option<T>) {
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


impl ORM {
    pub fn connect(url: String) -> Result<Arc<ORM>, ORMError> {
        let conn = Connection::open(url)?;
        Ok(Arc::new(ORM { conn }))
    }
    pub fn insert<T>(&self, data: T) -> QueryBuilder<i64, T>
    where T: TableDeserialize + TableSerialize + Serialize + 'static
    {
        let table_name = data.name();
        let types = serializer_types::to_string(&data).unwrap();
        let values = serializer_values::to_string(&data).unwrap();
        let query: String = format!("insert into {table_name} {types} values {values}");
        let qb = QueryBuilder::<i64,T> {
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

        let qb = QueryBuilder::<Option<T>, T> {
            query,
            entity: None,
            orm: self,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn findMany<T>(&self, query_where: String) -> QueryBuilder<Vec<T>, T>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static

    {

        let table_name = T::same_name();

        let query: String = format!("select * from {table_name} where {query_where}");

        let qb = QueryBuilder::<Vec<T>, T> {
            query,
            entity: None,
            orm: self,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn findAll<T>(&self) -> QueryBuilder<Vec<T>, T>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static {
        let table_name = T::same_name();

        let query: String = format!("select * from {table_name}");

        let qb = QueryBuilder::<Vec<T>, T> {
            query,
            entity: None,
            orm: self,
            phantom: std::marker::PhantomData,
        };
        qb
    }

    pub fn update<T>(&self, data: T, query_where: String) -> QueryBuilder<usize, ()>
        where T: TableDeserialize + TableSerialize + Serialize + 'static
    {
        let table_name = data.name();
        let key_value_str = serializer_key_values::to_string(&data).unwrap();
        // remove first and last char
        let key_value = &key_value_str[1..key_value_str.len()-1];
        let query: String = format!("update {table_name} set {key_value} where {query_where}");
        let qb = QueryBuilder::<usize, ()> {
            query,
            entity: None,
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
        let updated_rows: usize = self.query_update(query).exec().await?;

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
    pub async fn exec(&self) -> Result<usize, ORMError> {
        log::debug!("{}", self.query);
        let r = self.orm.conn.execute(self.query.as_str(),(),)?;
        Ok(r)
    }
}

impl<T> QueryBuilder<'_, i64,T> {
    pub async fn run(&self) -> Result<i64, ORMError> {
        log::debug!("{}", self.query);
        let _r = self.orm.conn.execute(self.query.as_str(),(),)?;
        let r = self.orm.last_insert_rowid();
        Ok(r)
    }
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

        let rows  = self.orm.query(self.query.clone()).exec().await?;
        let columns: Vec<String> =T::fields();
        if rows.len() == 0 {
            return Ok(None);
        } else {
            let mut column_str: Vec<String> = Vec::new();
            for row in rows {
                let mut i = 0;
                for column in columns.iter() {
                    let value_opt:Option<String> = row.get(i);
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
            let user_str = format!("{{{}}}", column_str.join(","));
            let user: T = deserializer_key_values::from_str(&user_str).unwrap();
            Ok(Some(user))

        }

    }
}

impl<R> QueryBuilder<'_, Vec<Row>,R> {
    pub async fn exec(&self) -> Result<Vec<Row>, ORMError>
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
                        r.set(i.try_into().unwrap(), Some(v));

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
                        r.set(i.try_into().unwrap(), Some(v));
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


}

impl<T> QueryBuilder<'_, Vec<T>,T> {
    pub async fn run(&self) -> Result<Vec<T>, ORMError>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static
    {

        let mut result: Vec<T> = Vec::new();
        let rows  = self.orm.query(self.query.clone()).exec().await?;
        let columns: Vec<String> =T::fields();
            for row in rows {
                let mut column_str: Vec<String> = Vec::new();
                let mut i = 0;
                for column in columns.iter() {
                    let value_opt:Option<String> = row.get(i);
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
                let user_str = format!("{{{}}}", column_str.join(","));
                let user: T = deserializer_key_values::from_str(&user_str).unwrap();
                result.push(user);
            }

            Ok(result)
    }

    pub fn limit(&self, limit: i32) -> QueryBuilder<Vec<T>, T> {

        let qb =  QueryBuilder::<Vec<T>,T> {
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