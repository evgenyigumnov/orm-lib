use std::fmt::Debug;
use std::sync::Arc;
use async_trait::async_trait;
use futures::lock::Mutex;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use crate::{deserializer_key_values, ORMError, ORMTrait, QueryBuilder, Row, serializer_error, serializer_key_values, serializer_types, serializer_values, TableDeserialize, TableSerialize};

#[derive(Debug)]
pub struct ORM {
    conn: Mutex<Option<Connection>>,
}

impl ORM {

    pub fn connect(url: String) -> Result<Arc<ORM>, ORMError>
        where Arc<ORM>: Send + Sync + 'static
    {
        let conn = Connection::open(url)?;
        Ok(Arc::new(ORM {
            conn: Mutex::new(Some(conn)),
        }))
    }
}
#[async_trait]
impl ORMTrait<ORM> for ORM {

    fn add<T>(&self, data: T) -> QueryBuilder<T, T, ORM>
        where T: for<'a> Deserialize<'a> + TableDeserialize + TableSerialize + Serialize + Debug + 'static
    {
        let table_name = data.name();
        let types = serializer_types::to_string(&data).unwrap();
        let values = serializer_values::to_string(&data).unwrap();
        let query: String = format!("insert into {table_name} {types} values {values}");
        let qb = QueryBuilder::<T,T, ORM> {
            query: query,
            entity: Default::default(),
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }

    async fn last_insert_rowid(&self)  -> Result<i64, ORMError>{
        let conn = self.conn.lock().await;
        if conn.is_none() {
            return Err(ORMError::NoConnection);
        }
        Ok(conn.as_ref().unwrap().last_insert_rowid())
    }

    async fn close(&self)  -> Result<(), ORMError>{
        let mut conn_lock = self.conn.lock().await;
        if conn_lock.is_none() {
            return Err(ORMError::NoConnection);
        }
        let conn = conn_lock.take();
        let r = conn.unwrap().close();
        match r {
            Ok(_) => {
                Ok(())
            }
            Err(e) => {
                Err(ORMError::RusqliteError(e.1))
            }
        }
    }

    fn find_one<T: TableDeserialize>(&self, id: u64) -> QueryBuilder<Option<T>, T, ORM>
        where T: TableDeserialize + TableSerialize + for<'a> Deserialize<'a> + 'static
    {
        let table_name = T::same_name();

        let query: String = format!("select * from {table_name} where id = {id}");

        let qb = QueryBuilder::<Option<T>, T, ORM> {
            query,
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }

    fn find_many<T>(&self, query_where: &str) -> QueryBuilder<Vec<T>, T, ORM>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static

    {

        let table_name = T::same_name();

        let query: String = format!("select * from {table_name} where {query_where}");

        let qb = QueryBuilder::<Vec<T>, T, ORM> {
            query,
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }

    fn find_all<T>(&self) -> QueryBuilder<Vec<T>, T, ORM>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static {
        let table_name = T::same_name();

        let query: String = format!("select * from {table_name}");

        let qb = QueryBuilder::<Vec<T>, T, ORM> {
            query,
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }

    fn modify<T>(&self, data: T) -> QueryBuilder<usize, (), ORM>
        where T: TableDeserialize + TableSerialize + Serialize + 'static
    {
        let table_name = data.name();
        let key_value_str = serializer_key_values::to_string(&data).unwrap();
        // remove first and last char
        let key_value = &key_value_str[1..key_value_str.len()-1];
        let id = data.get_id();
        let query: String = format!("update {table_name} set {key_value} where id = {id}");
        let qb = QueryBuilder::<usize, (), ORM> {
            query,
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }

    fn remove<T>(&self, data: T) -> QueryBuilder<usize, (), ORM>
        where T: TableDeserialize + TableSerialize + Serialize + 'static
    {
        let table_name = data.name();
        let id = data.get_id();
        let query: String = format!("delete from {table_name} where id = {id}");
        let qb = QueryBuilder::<usize, (), ORM> {
            query,
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }

    fn query<T>(&self, query: &str) -> QueryBuilder<Vec<T>, T, ORM> {
        let qb = QueryBuilder::<Vec<T>, T, ORM> {
            query: query.to_string(),
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }

    fn query_update(&self, query: &str) -> QueryBuilder<usize, (), ORM> {
        let qb = QueryBuilder::<usize, (), ORM> {
            query: query.to_string(),
            entity: std::marker::PhantomData,
            orm: self,
            result: std::marker::PhantomData,
        };
        qb
    }

    fn protect(&self, value: &str) -> String {
        let protected: String = format!("\"{}\"", ORM::escape(value));
        protected

    }
    fn escape(str: &str) -> String {
        let mut escaped = String::new();

        for c in str.chars() {
            match c {
                // '\'' => escaped.push_str("\\'"),
                '"' => escaped.push_str("\"\""),
                // '\\' => escaped.push_str("\\\\"),
                // '\n' => escaped.push_str("\\n"),
                // '\r' => escaped.push_str("\\r"),
                // '\t' => escaped.push_str("\\t"),
                // '\x08' => escaped.push_str("\\b"),
                // '\x0C' => escaped.push_str("\\f"),
                _ => escaped.push(c),
            }
        }

        escaped
    }

    fn escape_json(input: &str) -> String {
        let input = input.to_string();
        let mut escaped = input.clone();
        escaped = escaped.replace("\\", "\\\\");
        escaped = escaped.replace("\"", "\\\"");
        // escaped = escaped.replace("\\\"\\\\\"", "\\\"\\\"");

        // for c in input.chars() {
        //     match c {
        //         '"' => escaped.push_str("\\\""),
        //         // '\\' => escaped.push_str("\\\\"),
        //         // '\n' => escaped.push_str("\\n"),
        //         // '\r' => escaped.push_str("\\r"),
        //         // '\t' => escaped.push_str("\\t"),
        //         // '\x08' => escaped.push_str("\\b"),
        //         // '\x0C' => escaped.push_str("\\f"),
        //         _ => escaped.push(c),
        //     }
        // }
        escaped
    }


    async fn init(&self, script: &str) -> Result<(), ORMError>  {
        let query = std::fs::read_to_string(script)?;
        let _updated_rows: usize = self.query_update(query.as_str()).exec().await?;

        Ok(())
    }
}

impl<T> QueryBuilder<'_, usize, T, ORM>{
    pub async fn exec(&self) -> Result<usize, ORMError> {
        log::debug!("{:?}", self.query);
        let conn = self.orm.conn.lock().await;
        if conn.is_none() {
            return Err(ORMError::NoConnection);
        }
        let conn = conn.as_ref().unwrap();
        let r = conn.execute(self.query.as_str(),(),)?;
        Ok(r)
    }
}

impl<T> QueryBuilder<'_, T,T, ORM>{
    pub async fn apply(&self) -> Result<T, ORMError>
        where T: for<'a> Deserialize<'a> + TableDeserialize + TableSerialize + Debug + 'static
    {
        log::debug!("{:?}", self.query);
        let r = {
            let conn = self.orm.conn.lock().await;
            if conn.is_none() {
                return Err(ORMError::NoConnection);
            }
            let conn = conn.as_ref().unwrap();
            let _r = conn.execute(self.query.as_str(),(),)?;
            let r = conn.last_insert_rowid();
            r
        };
        let rows: Vec<T> = self.orm.find_many(format!("rowid = {}", r).as_str()).run().await?;
        if rows.len() == 0 {
            return Err(ORMError::InsertError);
        }
        let t_opt = rows.into_iter().next();
        match t_opt {
            Some(t) => Ok(t),
            None => Err(ORMError::InsertError),
        }

    }
}

impl<T> QueryBuilder<'_, usize,T, ORM> {
    pub async fn run(&self) -> Result<usize, ORMError> {
        log::debug!("{:?}", self.query);
        let conn = self.orm.conn.lock().await;
        if conn.is_none() {
            return Err(ORMError::NoConnection);
        }
        let conn = conn.as_ref().unwrap();
        let r = conn.execute(self.query.as_str(),(),)?;
        Ok(r)
    }
}


impl<T> QueryBuilder<'_, Option<T>,T, ORM>
    where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static
{
    pub async fn run(&self) -> Result<Option<T>, ORMError> {

        let rows  = self.orm.query(self.query.clone().as_str()).exec().await?;
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
                            format!("\"{}\"", ORM::escape_json(v.as_str()))
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
            // log::debug!("zzz{}", user_str);
            let user: T = deserializer_key_values::from_str(&user_str).unwrap();
            Ok(Some(user))

        }

    }
}

impl<R> QueryBuilder<'_, Vec<Row>,R, ORM> {
    pub async fn exec(&self) -> Result<Vec<Row>, ORMError>
    {
        log::debug!("{:?}", self.query);
        let conn = self.orm.conn.lock().await;
        if conn.is_none() {
            return Err(ORMError::NoConnection);
        }
        let conn = conn.as_ref().unwrap();
        let stmt_result = conn.prepare( self.query.as_str());
        if stmt_result.is_err() {
            let e = stmt_result.err().unwrap();
            log::error!("{:?}", e);
            return Err(ORMError::RusqliteError(e));
        }
        let mut stmt = stmt_result.unwrap();
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
                    Err(_e) => {
                    }
                }

                i = i + 1;
            }

            result.push(r);
            Ok(())
        })?;
        for _x in person_iter {
        }
        // log::debug!("{:?}", result);

        Ok(result)
    }


}

impl<T> QueryBuilder<'_, Vec<T>,T, ORM> {
    pub async fn run(&self) -> Result<Vec<T>, ORMError>
        where T: for<'a> Deserialize<'a> + TableDeserialize + Debug + 'static
    {

        let mut result: Vec<T> = Vec::new();
        let rows  = self.orm.query(self.query.clone().as_str()).exec().await?;
        let columns: Vec<String> =T::fields();
        for row in rows {
            let mut column_str: Vec<String> = Vec::new();
            let mut i = 0;
            for column in columns.iter() {
                let value_opt:Option<String> = row.get(i);
                let value = match value_opt {
                    Some(v) => {
                        format!("\"{}\"", ORM::escape_json(v.as_str()))
                    }
                    None => {
                        "null".to_string()
                    }
                };
                column_str.push(format!("\"{}\":{}", column, value));
                i = i + 1;
            }
            let user_str = format!("{{{}}}", column_str.join(","));
            // log::info!("{}", user_str);
            let user_result: std::result::Result<T, serializer_error::Error> = deserializer_key_values::from_str(&user_str);
            match user_result {
                Ok(user) => {
                    result.push(user);
                }
                Err(e) => {
                    log::error!("{:?}", e);
                    log::error!("{}", user_str);
                    return Err(ORMError::Unknown);
                }
            }

        }

        Ok(result)
    }

    pub fn limit(&self, limit: i32) -> QueryBuilder<Vec<T>, T, ORM> {

        let qb =  QueryBuilder::<Vec<T>,T, ORM> {
            query: format!("{} limit {}", self.query, limit),
            entity: std::marker::PhantomData,
            orm: self.orm,
            result: std::marker::PhantomData,
        };
        qb
    }
}

