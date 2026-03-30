use thiserror::Error;

#[derive(Debug, Error)]
pub enum MocoError {
    #[error("database error: {0}")]
    Database(#[from] redb::Error),

    #[error("database error: {0}")]
    DatabaseOpen(#[from] redb::DatabaseError),

    #[error("database transaction error: {0}")]
    DatabaseTransaction(#[from] redb::TransactionError),

    #[error("database table error: {0}")]
    DatabaseTable(#[from] redb::TableError),

    #[error("database storage error: {0}")]
    DatabaseStorage(#[from] redb::StorageError),

    #[error("database commit error: {0}")]
    DatabaseCommit(#[from] redb::CommitError),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("project already initialized at this path (use --force to reinitialize)")]
    AlreadyInitialized,

    #[error("no project found; run `moco init <name>` to initialize one")]
    ProjectNotFound,

    #[error("task #{0} not found")]
    TaskNotFound(u32),

    #[error("invalid status value '{0}': expected 0–100, 'complete', or 'defer'")]
    InvalidStatus(String),

    #[error("home directory could not be determined")]
    HomeNotFound,
}
