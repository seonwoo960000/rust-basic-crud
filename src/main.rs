use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts, State},
    http::{request::Parts, StatusCode},
    routing::get,
    Router,
};
use sqlx::postgres::{PgPool, PgPoolOptions};
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use std::time::Duration;
use chrono::NaiveDateTime;
use sqlx::{FromRow, Row};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_tokio_postgres=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_connection_str = "postgres://frustacean:abc123@localhost:5434/community".to_string();

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&db_connection_str)
        .await
        .expect("can't connect to db");


    let account = match sqlx::query_as!(Account,
        r#"INSERT INTO account (uuid, name, status, hashed_password)
           VALUES ($1, $2, $3, $4)
           RETURNING id, uuid, name, status AS "status!: AccountStatus", hashed_password, created_at, updated_at
        "#,
        Uuid::new_v4(),
        Uuid::new_v4().to_string() + "test",
        AccountStatus::Active as AccountStatus,
        "hashed_password")
        .fetch_one(&pool)
        .await
        .map_err(internal_error) {
        Ok(account) => account,
        Err(e) => {
            println!("error: {:?}", e);
            return;
        }
    };

    let result = sqlx::query!(
        r#"UPDATE account SET name = $1 WHERE id = $2"#,
        Uuid::new_v4().to_string() + "new name",
        account.id
    ).execute(&pool)
        .await
        .unwrap();
    println!("result: {:?}", result);


    let inserted_with_date = sqlx::query_as!(Account,
        r#"INSERT INTO account (uuid, name, status, hashed_password, created_at, updated_at)
           VALUES ($1, $2, $3, $4, $5, $6)
           RETURNING id, uuid, name, status AS "status!: AccountStatus", hashed_password, created_at, updated_at
        "#,
        Uuid::new_v4(),
        Uuid::new_v4().to_string() + "test",
        AccountStatus::Active as AccountStatus,
        "hashed_password",
        NaiveDateTime::from_timestamp(0, 0),
        NaiveDateTime::from_timestamp(0, 0))
        .fetch_one(&pool)
        .await
        .map_err(internal_error);
    println!("inserted with date: {:?}", inserted_with_date);


    let accounts = sqlx::query("select * from account")
        .fetch_all(&pool)
        .await
        .map(|accounts| {
            accounts
                .into_iter()
                .map(|row| {
                    let id: i64 = row.get("id");
                    let uuid: Uuid = row.get("uuid");
                    let name: String = row.get("name");
                    let status: AccountStatus = row.get("status");
                    let hashed_password: String = row.get("hashed_password");
                    let created_at: NaiveDateTime = row.get("created_at");
                    let updated_at: NaiveDateTime = row.get("updated_at");
                    Account {
                        id,
                        uuid,
                        name,
                        status,
                        hashed_password,
                        created_at,
                        updated_at,
                    }
                })
                .collect::<Vec<Account>>()
        })
        .map_err(internal_error);

    println!("val: {:?}", accounts);
}

fn internal_error<E>(err: E) -> (StatusCode, String)
    where
        E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}

#[derive(sqlx::Type, Debug, Serialize, Deserialize)]
#[sqlx(type_name = "account_status", rename_all = "lowercase")]
pub enum AccountStatus {
    Active,
    Deleted,
    Abnormal,
}

#[derive(Debug, FromRow, Serialize, Deserialize)]
pub struct Account {
    pub(crate) id: i64,
    pub(crate) uuid: Uuid,
    pub(crate) name: String,
    pub(crate) status: AccountStatus,
    pub(crate) hashed_password: String,
    pub(crate) created_at: NaiveDateTime,
    pub(crate) updated_at: NaiveDateTime,
}

impl Account {}