use rusqlite::Connection;
use sea_query::{ColumnDef, Expr, OnConflict, Query, SqliteQueryBuilder, Table};
use sea_query_rusqlite::RusqliteBinder;
use crate::db::sqlite::schema;
use crate::db::sqlite::schema::EventType;

pub struct Client {
    conn: Connection,
}

impl Client {
    pub fn new() -> Self {
        let conn = Connection::open(format!("{}.db", crate::APP_NAME)).expect("unable to connect db");

        let init_schema = Table::create()
            .table(schema::EventIden::Table)
            .if_not_exists()
            .col(ColumnDef::new(schema::EventIden::ID)
                .integer()
                .auto_increment()
                .primary_key()
            )
            .col(ColumnDef::new(schema::EventIden::ChatID).integer().not_null())
            .col(ColumnDef::new(schema::EventIden::Type).text().not_null())
            .col(ColumnDef::new(schema::EventIden::Meta).text().null())
            .build(SqliteQueryBuilder);

        conn.execute(&init_schema, []).expect("unable to init schema");

        Self { conn }
    }

    pub fn add_event(&self, e: schema::Event) -> Result<(), String> {
        let (sql, params) = Query::insert()
            .into_table(schema::EventIden::Table)
            .columns([
                schema::EventIden::ChatID,
                schema::EventIden::Type,
                schema::EventIden::User,
                schema::EventIden::Meta,
            ])
            .values_panic([
                e.chat_id.into(),
                e.typ.to_string().into(),
                e.user.into(),
                e.meta.into(),
            ])
            .on_conflict(OnConflict::new().do_nothing().to_owned())
            .build_rusqlite(SqliteQueryBuilder);

        self.conn
            .execute(&sql, params.as_params().as_slice())
            .map_err(|err| format!("unable to insert event: {err}"))?;

        Ok(())
    }

    pub fn delete_event(&self, chat_id: i64, typ: EventType) -> Result<(), String> {
        let (sql, params) = Query::delete()
            .from_table(schema::EventIden::Table)
            .and_where(Expr::col(schema::EventIden::ChatID).eq(chat_id))
            .and_where(Expr::col(schema::EventIden::Type).eq(typ.to_string()))
            .build_rusqlite(SqliteQueryBuilder);

        self.conn
            .execute(&sql, params.as_params().as_slice())
            .map_err(|err| format!("unable to delete event with chat_id='{chat_id}': {err}"))?;

        Ok(())
    }
}
