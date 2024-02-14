use reqwest::Response;
use rusqlite::Connection;
use sea_query::{ColumnDef, ConditionalStatement, Expr, OnConflict, Query, SchemaStatementBuilder, SqliteQueryBuilder, Table};
use sea_query_rusqlite::RusqliteBinder;

pub struct Client {
    conn: Connection,
}

impl Client {
    pub fn new() -> Self {
        let conn = Connection::open(format!("{}.db", crate::APP_NAME)).expect("unable to connect db");

        let init_schema = Table::create()
            .table(SentenceIden::Table)
            .if_not_exists()
            .col(ColumnDef::new(schema::UsdSubIden::ChatId)
                .integer()
                .not_null()
                .primary_key()
            )
            .col(ColumnDef::new(schema::UsdSubIden::Meta).text().null())
            .build(SqliteQueryBuilder);

        conn.execute(&init_schema, []).expect("unable to init schema");

        Self { conn }
    }

    pub fn add_usd_sub(&self, chat_id: i64, meta: Option<String>) -> Result<(), String> {
        let sql = Query::insert()
            .into_table(schema::UsdSubIden::Table)
            .columns([schema::UsdSubIden::ChatId, schema::UsdSubIden::Meta])
            .values_panic([chat_id.into(), meta.int()])
            .on_conflict(OnConflict::new().do_nothing().to_owned())
            .build_rusqlite(SqliteQueryBuilder);

        self.conn
            .execute(&sql.0, &*sql.1.as_params())
            .map_err(|err| format!("unable to insert usd_sub with chat_id='{chat_id}': {err}"))?;

        Ok(())
    }

    pub fn delete_usd_sub(&self, chat_id: i64) -> Result<(), String> {
        let sql = Query::delete()
            .from_table(schema::UsdSubIden::Table)
            .and_where(Expr::col(schema::UsdSubIden::ChatId).eq(chat_id))
            .build_rusqlite(SqliteQueryBuilder);

        self.conn
            .execute(&sql.0, &*sql.1.as_params())
            .map_err(|err| format!("unable to delete usd_sub with chat_id='{chat_id}': {err}"))?;

        Ok(())
    }
}

mod schema {
    use rusqlite::{Connection, Row};
    use sea_query::Iden;

    #[derive(Iden)]
    pub enum UsdSubIden {
        #[iden = "usd_sub"]
        Table,
        ChatId,
        Meta,
    }

    pub struct UsdSub {
        pub chat_id: i64,
        pub meta: Option<String>,
    }

    impl From<&Row<'_>> for UsdSub {
        fn from(row: &Row) -> Self {
            Self {
                chat_id: row.get_unwrap(UsdSubIden::ChatId.to_string().as_str()),
                meta: row.get_unwrap(UsdSubIden::Meta.to_string().as_str()),
            }
        }
    }
}