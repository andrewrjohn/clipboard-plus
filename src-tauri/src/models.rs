use chrono::{DateTime, NaiveDateTime, Utc};
use diesel::{
    prelude::{Insertable, Queryable},
    sql_types::Timestamp,
    Selectable,
};
use serde::Serialize;
use specta::Type;

#[derive(Queryable, Selectable, Serialize, Type)]
#[diesel(table_name = crate::schema::items)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Item {
    pub id: i32,
    pub text: Option<String>,
    pub image: Option<String>,
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub timestamp: i64,
    pub size_bytes: i32,
    pub source_app: Option<String>,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schema::items)]
pub struct NewItem<'a> {
    pub text: Option<&'a str>,
    pub image: Option<&'a str>,
    pub image_width: Option<i32>,
    pub image_height: Option<i32>,
    pub timestamp: i64,
    pub size_bytes: i32,
    pub source_app: Option<&'a str>,
}
