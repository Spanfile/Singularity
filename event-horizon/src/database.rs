pub mod models;
pub mod schema;

use diesel::{
    r2d2::{self, ConnectionManager},
    SqliteConnection,
};

pub type DbConn = SqliteConnection;
pub type DbPool = r2d2::Pool<ConnectionManager<DbConn>>;
/// The ID type used across the database schema
pub type DbId = i32;

pub type RedisPool = r2d2::Pool<redis::Client>;
