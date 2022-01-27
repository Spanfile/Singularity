pub mod models;
pub mod schema;

use crate::error::{EvhError, EvhResult};
use diesel::{
    r2d2::{self, ConnectionManager},
    SqliteConnection,
};

pub type DbConn = SqliteConnection;
/// The ID type used across the database schema
pub type DbId = i32;

pub type RedisPool = r2d2::Pool<redis::Client>;

pub struct DbPool(r2d2::Pool<ConnectionManager<DbConn>>);

impl DbPool {
    pub fn new(pool: r2d2::Pool<ConnectionManager<DbConn>>) -> Self {
        Self(pool)
    }

    pub fn get(&self) -> EvhResult<r2d2::PooledConnection<r2d2::ConnectionManager<DbConn>>> {
        self.0.get().map_err(EvhError::DatabaseConnectionAcquireFailed)
    }
}
