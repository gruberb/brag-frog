use serde::{Deserialize, Serialize};

use crate::kernel::crypto::UserCrypto;
use crate::kernel::error::AppError;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TodoRow {
    pub id: i64,
    pub user_id: i64,
    pub title: Vec<u8>,
    pub completed: i64,
    pub sort_order: i64,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub completed: i64,
    pub sort_order: i64,
    pub created_at: String,
    pub completed_at: Option<String>,
}

impl TodoRow {
    pub fn decrypt(self, crypto: &UserCrypto) -> Result<Todo, AppError> {
        Ok(Todo {
            id: self.id,
            user_id: self.user_id,
            title: crypto.decrypt(&self.title)?,
            completed: self.completed,
            sort_order: self.sort_order,
            created_at: self.created_at,
            completed_at: self.completed_at,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateTodo {
    pub title: String,
}
