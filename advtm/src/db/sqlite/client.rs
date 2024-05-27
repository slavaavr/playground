use rusqlite::Connection;
use sea_query::{ColumnDef, Expr, OnConflict, Query, SqliteQueryBuilder, Table};
use sea_query_rusqlite::RusqliteBinder;
use crate::db::sqlite::schema::{Event, EventIden, EventType};

pub struct Client {
    conn: Connection,
}

impl Client {
    pub fn new() -> Self {
        let conn = Connection::open(format!("{}.db", crate::APP_NAME)).expect("unable to connect db");

        let init_schema = Table::create()
            .table(EventIden::Table)
            .if_not_exists()
            .col(ColumnDef::new(EventIden::ID)
                .integer()
                .auto_increment()
                .primary_key()
            )
            .col(ColumnDef::new(EventIden::ChatID).integer().not_null())
            .col(ColumnDef::new(EventIden::Type).text().not_null())
            .col(ColumnDef::new(EventIden::User).text().not_null())
            .col(ColumnDef::new(EventIden::Meta).text().null())
            .build(SqliteQueryBuilder);

        conn.execute(&init_schema, []).expect("unable to init schema");

        Self { conn }
    }

    pub fn list_events(&self) -> Vec<Event> {
        let (sql, params) = Query::select()
            .from(EventIden::Table)
            .columns([
                EventIden::ID,
                EventIden::ChatID,
                EventIden::Type,
                EventIden::User,
                EventIden::Meta,
            ])
            .build_rusqlite(SqliteQueryBuilder);


        let mut stmt = self.conn.prepare(&sql).unwrap();
        let mut rows = stmt.query(params.as_params().as_slice()).unwrap();

        let mut res = Vec::new();

        while let Some(row) = rows.next().unwrap() {
            res.push(Event::from(row));
        }

        return res;
    }

    pub fn get_event(&self, chat_id: i64, typ: EventType) -> Option<Event> {
        let (sql, params) = Query::select()
            .from(EventIden::Table)
            .columns([
                EventIden::ID,
                EventIden::ChatID,
                EventIden::Type,
                EventIden::User,
                EventIden::Meta,
            ])
            .and_where(Expr::col(EventIden::ChatID).eq(chat_id))
            .and_where(Expr::col(EventIden::Type).eq(typ.to_string()))
            .build_rusqlite(SqliteQueryBuilder);

        let mut stmt = self.conn.prepare(&sql).unwrap();
        stmt.query_row(params.as_params().as_slice(), |row | Ok(Event::from(row))).ok()
    }

    pub fn add_event(&self, e: Event) -> Result<(), String> {
        let (sql, params) = Query::insert()
            .into_table(EventIden::Table)
            .columns([
                EventIden::ChatID,
                EventIden::Type,
                EventIden::User,
                EventIden::Meta,
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
            .from_table(EventIden::Table)
            .and_where(Expr::col(EventIden::ChatID).eq(chat_id))
            .and_where(Expr::col(EventIden::Type).eq(typ.to_string()))
            .build_rusqlite(SqliteQueryBuilder);

        self.conn
            .execute(&sql, params.as_params().as_slice())
            .map_err(|err| format!("unable to delete event with chat_id='{chat_id}': {err}"))?;

        Ok(())
    }
}
