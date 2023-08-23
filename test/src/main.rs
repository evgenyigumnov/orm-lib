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
}

