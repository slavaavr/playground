use std::fmt::{Display, Formatter};
use rusqlite::{Row, ToSql};
use rusqlite::types::{FromSql, FromSqlResult, ToSqlOutput, ValueRef};
use sea_query::Iden;

#[derive(Iden)]
pub enum EventIden {
    #[iden = "event"]
    Table,
    ID,
    ChatID,
    Type,
    User,
    Meta,
}

#[derive(Clone)]
pub struct Event {
    pub id: i64,
    pub chat_id: i64,
    pub typ: EventType,
    pub user: Option<String>,
    pub meta: Option<String>,
}

#[derive(Clone)]
pub enum EventType {
    UsdSubscription,
    LevadaSubscription,
    StandupSubscription,
}

impl Display for EventType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            EventType::UsdSubscription => "usd_subscription",
            EventType::LevadaSubscription => "levada_subscription",
            EventType::StandupSubscription => "standup_subscription",
        };

        write!(f, "{}", s)
    }
}

impl From<String> for EventType {
    fn from(value: String) -> Self {
        match value.as_str() {
            "usd_subscription" => EventType::UsdSubscription,
            "levada_subscription" => EventType::LevadaSubscription,
            "standup_subscription" => EventType::StandupSubscription,
            _ => EventType::UsdSubscription,
        }
    }
}

impl FromSql for EventType {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        Ok(value.as_str()?.to_string().into())
    }
}

impl ToSql for EventType {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::Owned(self.to_string().into()))
    }
}

impl From<&Row<'_>> for Event {
    fn from(row: &Row) -> Self {
        Self {
            id: row.get_unwrap(EventIden::ID.to_string().as_str()),
            chat_id: row.get_unwrap(EventIden::ChatID.to_string().as_str()),
            typ: row.get_unwrap(EventIden::Type.to_string().as_str()),
            user: row.get_unwrap(EventIden::User.to_string().as_str()),
            meta: row.get_unwrap(EventIden::Meta.to_string().as_str()),
        }
    }
}