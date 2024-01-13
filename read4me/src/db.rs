pub mod sqlite {
    use rusqlite::{Connection, Row};
    use sea_query::{
        ColumnDef,
        ConditionalStatement,
        Expr,
        Iden,
        Order,
        OrderedStatement,
        Query,
        SchemaStatementBuilder,
        SqliteQueryBuilder,
        Table,
    };
    use sea_query_rusqlite::RusqliteBinder;

    #[derive(Iden)]
    enum SentenceIden {
        #[iden = "sentence"]
        Table,
        Id,
        Text,
        Uri,
    }

    pub struct Sentence {
        pub id: i32,
        pub text: String,
        pub uri: Option<String>,
    }

    impl From<&Row<'_>> for Sentence {
        fn from(row: &Row) -> Self {
            Self {
                id: row.get_unwrap(SentenceIden::Id.to_string().as_str()),
                text: row.get_unwrap(SentenceIden::Text.to_string().as_str()),
                uri: row.get_unwrap(SentenceIden::Uri.to_string().as_str()),
            }
        }
    }

    pub struct Client {
        conn: Connection,
    }

    impl Client {
        pub fn new() -> Self {
            let conn = Connection::open(format!("{}.db", crate::APP_NAME)).expect("unable to connect db");

            let init_schema = Table::create()
                .table(SentenceIden::Table)
                .if_not_exists()
                .col(
                    ColumnDef::new(SentenceIden::Id)
                        .integer()
                        .not_null()
                        .auto_increment()
                        .primary_key()
                )
                .col(ColumnDef::new(SentenceIden::Text).text().not_null())
                .col(ColumnDef::new(SentenceIden::Uri).text().null())
                .build(SqliteQueryBuilder);

            conn.execute(&init_schema, []).expect("unable to init schema");

            Self { conn }
        }

        pub fn add_sentence(&self, text: String) -> Result<i32, String> {
            let sql = Query::insert()
                .into_table(SentenceIden::Table)
                .columns([SentenceIden::Text])
                .values_panic([text.into()])
                .build_rusqlite(SqliteQueryBuilder);


            let mut stmt = self.conn.prepare(&sql.0).expect("unable to prepare stmt");
            let id = stmt.insert(&*sql.1.as_params())
                .map_err(|err| format!("unable to insert sentence: {err}"))?;

            Ok(id as i32)
        }

        pub fn drop_sentence(&self, id: i32) -> Result<(), String> {
            let sql = Query::delete()
                .from_table(SentenceIden::Table)
                .and_where(Expr::col(SentenceIden::Id).eq(id))
                .build_rusqlite(SqliteQueryBuilder);

            self.conn
                .execute(&sql.0, &*sql.1.as_params())
                .map_err(|err| format!("unable to drop sentence id='{id}: {err}'"))?;

            Ok(())
        }

        pub fn get_sentence(&self, id: i32) -> Result<Sentence, String> {
            let sql = Query::select()
                .columns([SentenceIden::Id, SentenceIden::Text, SentenceIden::Uri])
                .from(SentenceIden::Table)
                .and_where(Expr::col(SentenceIden::Id).eq(id))
                .build_rusqlite(SqliteQueryBuilder);

            let mut stmt = self.conn.prepare(sql.0.as_str()).expect("unable to prepare stmt");
            let res = stmt.query_row(&*sql.1.as_params(), |row| Ok(Sentence::from(row)))
                .map_err(|err| format!("unable to get sentence id='{id}': {err}"))?;

            Ok(res)
        }

        pub fn list_sentences(&self) -> Result<Vec<Sentence>, String> {
            let sql = Query::select()
                .columns([SentenceIden::Id, SentenceIden::Text, SentenceIden::Uri])
                .from(SentenceIden::Table)
                .order_by(SentenceIden::Id, Order::Desc)
                .build_rusqlite(SqliteQueryBuilder);

            let mut stmt = self.conn.prepare(sql.0.as_str()).expect("unable to prepare stmt");
            let mut rows = stmt.query(&*sql.1.as_params())
                .map_err(|err| format!("unable to list sentences: {err}"))?;

            let mut res = Vec::new();

            while let Some(row) = rows.next().map_err(|err| format!("unable to do next(): {err}"))? {
                res.push(Sentence::from(row));
            }

            Ok(res)
        }

        pub fn update_sentence_uri(&self, id: i32, uri: String) -> Result<(), String> {
            let sql = Query::update()
                .table(SentenceIden::Table)
                .value(SentenceIden::Uri, uri)
                .and_where(Expr::col(SentenceIden::Id).eq(id))
                .build_rusqlite(SqliteQueryBuilder);

            let mut stmt = self.conn.prepare(sql.0.as_str()).expect("unable to prepare stmt");
            stmt.execute(&*sql.1.as_params())
                .map_err(|err| format!("unable to update sentence with id={id}: {err}"))?;

            Ok(())
        }
    }
}
