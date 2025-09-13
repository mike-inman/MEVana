//! Database abstraction layer
//!
//! Provides a unified interface for database operations with:
//! - Repository pattern for testability
//! - Circuit breaker integration for resilience
//! - Connection pooling with automatic retry
//! - Type-safe query operations

pub mod repository;
pub mod connection;
pub mod migrations;
pub mod models;

pub use repository::{DatabaseRepository, SqlxRepository};
pub use connection::{DatabasePool, create_connection_pool};
pub use models::*;

use crate::error::{ApiError, ApiResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use uuid::Uuid;

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connection_timeout_seconds: u64,
    pub idle_timeout_seconds: u64,
    pub max_lifetime_seconds: u64,
    pub enable_logging: bool,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgresql://localhost:5432/mevana".to_string(),
            max_connections: 20,
            min_connections: 5,
            connection_timeout_seconds: 30,
            idle_timeout_seconds: 600,
            max_lifetime_seconds: 3600,
            enable_logging: true,
        }
    }
}

/// Generic trait for database entities
pub trait Entity: Send + Sync + Clone {
    type Id: Send + Sync + Clone;
    
    fn id(&self) -> Option<Self::Id>;
    fn set_id(&mut self, id: Self::Id);
    fn table_name() -> &'static str;
}

/// Audit trail information for all entities
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuditInfo {
    pub created_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Uuid,
    pub version: i64,
}

impl Default for AuditInfo {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            created_at: now,
            created_by: Uuid::nil(),
            updated_at: now,
            updated_by: Uuid::nil(),
            version: 1,
        }
    }
}

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    pub page: Option<u64>,
    pub limit: Option<u64>,
    pub sort_by: Option<String>,
    pub sort_order: Option<SortOrder>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(50),
            sort_by: None,
            sort_order: Some(SortOrder::Desc),
        }
    }
}

/// Paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total_count: u64,
    pub page: u64,
    pub limit: u64,
    pub total_pages: u64,
    pub has_next: bool,
    pub has_previous: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total_count: u64, page: u64, limit: u64) -> Self {
        let total_pages = (total_count + limit - 1) / limit;
        Self {
            items,
            total_count,
            page,
            limit,
            total_pages,
            has_next: page < total_pages,
            has_previous: page > 1,
        }
    }
}

/// Database query builder for complex queries
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    table: String,
    select_fields: Vec<String>,
    where_clauses: Vec<String>,
    joins: Vec<String>,
    order_by: Vec<String>,
    limit: Option<u64>,
    offset: Option<u64>,
    parameters: Vec<serde_json::Value>,
}

impl QueryBuilder {
    pub fn new(table: &str) -> Self {
        Self {
            table: table.to_string(),
            select_fields: vec!["*".to_string()],
            where_clauses: Vec::new(),
            joins: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            parameters: Vec::new(),
        }
    }
    
    pub fn select<F: ToString>(mut self, fields: &[F]) -> Self {
        self.select_fields = fields.iter().map(|f| f.to_string()).collect();
        self
    }
    
    pub fn where_clause(mut self, clause: &str) -> Self {
        self.where_clauses.push(clause.to_string());
        self
    }
    
    pub fn join(mut self, join: &str) -> Self {
        self.joins.push(join.to_string());
        self
    }
    
    pub fn order_by(mut self, field: &str, order: SortOrder) -> Self {
        let order_str = match order {
            SortOrder::Asc => "ASC",
            SortOrder::Desc => "DESC",
        };
        self.order_by.push(format!("{} {}", field, order_str));
        self
    }
    
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }
    
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }
    
    pub fn build(&self) -> String {
        let mut query = format!("SELECT {} FROM {}", 
            self.select_fields.join(", "), 
            self.table
        );
        
        if !self.joins.is_empty() {
            query.push_str(&format!(" {}", self.joins.join(" ")));
        }
        
        if !self.where_clauses.is_empty() {
            query.push_str(&format!(" WHERE {}", self.where_clauses.join(" AND ")));
        }
        
        if !self.order_by.is_empty() {
            query.push_str(&format!(" ORDER BY {}", self.order_by.join(", ")));
        }
        
        if let Some(limit) = self.limit {
            query.push_str(&format!(" LIMIT {}", limit));
        }
        
        if let Some(offset) = self.offset {
            query.push_str(&format!(" OFFSET {}", offset));
        }
        
        query
    }
}