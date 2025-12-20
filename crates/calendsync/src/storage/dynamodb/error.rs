//! DynamoDB error mapping.
//!
//! Maps AWS SDK errors to `RepositoryError` from `calendsync_core::storage`.

use std::fmt::Debug;

use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::delete_item::DeleteItemError;
use aws_sdk_dynamodb::operation::get_item::GetItemError;
use aws_sdk_dynamodb::operation::put_item::PutItemError;
use aws_sdk_dynamodb::operation::query::QueryError;
use aws_sdk_dynamodb::operation::update_item::UpdateItemError;
use calendsync_core::storage::RepositoryError;

/// Map a GetItem SDK error to RepositoryError.
pub fn map_get_item_error<R: Debug + Send + Sync + 'static>(
    err: SdkError<GetItemError, R>,
    entity_type: &'static str,
    id: impl Into<String>,
) -> RepositoryError {
    let id_str = id.into();
    match err.into_service_error() {
        GetItemError::ResourceNotFoundException(_) => RepositoryError::NotFound {
            entity_type,
            id: id_str,
        },
        GetItemError::ProvisionedThroughputExceededException(_) => {
            RepositoryError::QueryFailed("Throughput exceeded, please retry".to_string())
        }
        GetItemError::RequestLimitExceeded(_) => {
            RepositoryError::QueryFailed("Request limit exceeded, please retry".to_string())
        }
        GetItemError::InternalServerError(_) => {
            RepositoryError::QueryFailed("DynamoDB internal server error".to_string())
        }
        err => RepositoryError::QueryFailed(format!("GetItem failed: {:?}", err)),
    }
}

/// Map a Query SDK error to RepositoryError.
pub fn map_query_error<R: Debug + Send + Sync + 'static>(
    err: SdkError<QueryError, R>,
) -> RepositoryError {
    match err.into_service_error() {
        QueryError::ResourceNotFoundException(_) => {
            RepositoryError::QueryFailed("Table not found".to_string())
        }
        QueryError::ProvisionedThroughputExceededException(_) => {
            RepositoryError::QueryFailed("Throughput exceeded, please retry".to_string())
        }
        QueryError::RequestLimitExceeded(_) => {
            RepositoryError::QueryFailed("Request limit exceeded, please retry".to_string())
        }
        QueryError::InternalServerError(_) => {
            RepositoryError::QueryFailed("DynamoDB internal server error".to_string())
        }
        err => RepositoryError::QueryFailed(format!("Query failed: {:?}", err)),
    }
}

/// Map a PutItem SDK error to RepositoryError.
pub fn map_put_item_error<R: Debug + Send + Sync + 'static>(
    err: SdkError<PutItemError, R>,
    entity_type: &'static str,
    id: impl Into<String>,
) -> RepositoryError {
    let id_str = id.into();
    match err.into_service_error() {
        PutItemError::ConditionalCheckFailedException(_) => RepositoryError::AlreadyExists {
            entity_type,
            id: id_str,
        },
        PutItemError::ResourceNotFoundException(_) => {
            RepositoryError::QueryFailed("Table not found".to_string())
        }
        PutItemError::ProvisionedThroughputExceededException(_) => {
            RepositoryError::QueryFailed("Throughput exceeded, please retry".to_string())
        }
        PutItemError::RequestLimitExceeded(_) => {
            RepositoryError::QueryFailed("Request limit exceeded, please retry".to_string())
        }
        PutItemError::ItemCollectionSizeLimitExceededException(_) => {
            RepositoryError::QueryFailed("Item collection size limit exceeded".to_string())
        }
        PutItemError::TransactionConflictException(_) => {
            RepositoryError::QueryFailed("Transaction conflict, please retry".to_string())
        }
        PutItemError::InternalServerError(_) => {
            RepositoryError::QueryFailed("DynamoDB internal server error".to_string())
        }
        err => RepositoryError::QueryFailed(format!("PutItem failed: {:?}", err)),
    }
}

/// Map an UpdateItem SDK error to RepositoryError.
pub fn map_update_item_error<R: Debug + Send + Sync + 'static>(
    err: SdkError<UpdateItemError, R>,
    entity_type: &'static str,
    id: impl Into<String>,
) -> RepositoryError {
    let id_str = id.into();
    match err.into_service_error() {
        UpdateItemError::ConditionalCheckFailedException(_) => RepositoryError::NotFound {
            entity_type,
            id: id_str,
        },
        UpdateItemError::ResourceNotFoundException(_) => {
            RepositoryError::QueryFailed("Table not found".to_string())
        }
        UpdateItemError::ProvisionedThroughputExceededException(_) => {
            RepositoryError::QueryFailed("Throughput exceeded, please retry".to_string())
        }
        UpdateItemError::RequestLimitExceeded(_) => {
            RepositoryError::QueryFailed("Request limit exceeded, please retry".to_string())
        }
        UpdateItemError::ItemCollectionSizeLimitExceededException(_) => {
            RepositoryError::QueryFailed("Item collection size limit exceeded".to_string())
        }
        UpdateItemError::TransactionConflictException(_) => {
            RepositoryError::QueryFailed("Transaction conflict, please retry".to_string())
        }
        UpdateItemError::InternalServerError(_) => {
            RepositoryError::QueryFailed("DynamoDB internal server error".to_string())
        }
        err => RepositoryError::QueryFailed(format!("UpdateItem failed: {:?}", err)),
    }
}

/// Map a DeleteItem SDK error to RepositoryError.
pub fn map_delete_item_error<R: Debug + Send + Sync + 'static>(
    err: SdkError<DeleteItemError, R>,
    entity_type: &'static str,
    id: impl Into<String>,
) -> RepositoryError {
    let id_str = id.into();
    match err.into_service_error() {
        DeleteItemError::ConditionalCheckFailedException(_) => RepositoryError::NotFound {
            entity_type,
            id: id_str,
        },
        DeleteItemError::ResourceNotFoundException(_) => {
            RepositoryError::QueryFailed("Table not found".to_string())
        }
        DeleteItemError::ProvisionedThroughputExceededException(_) => {
            RepositoryError::QueryFailed("Throughput exceeded, please retry".to_string())
        }
        DeleteItemError::RequestLimitExceeded(_) => {
            RepositoryError::QueryFailed("Request limit exceeded, please retry".to_string())
        }
        DeleteItemError::ItemCollectionSizeLimitExceededException(_) => {
            RepositoryError::QueryFailed("Item collection size limit exceeded".to_string())
        }
        DeleteItemError::TransactionConflictException(_) => {
            RepositoryError::QueryFailed("Transaction conflict, please retry".to_string())
        }
        DeleteItemError::InternalServerError(_) => {
            RepositoryError::QueryFailed("DynamoDB internal server error".to_string())
        }
        err => RepositoryError::QueryFailed(format!("DeleteItem failed: {:?}", err)),
    }
}

/// Map a generic connection/config error to RepositoryError.
pub fn map_connection_error(err: impl std::fmt::Display) -> RepositoryError {
    RepositoryError::ConnectionFailed(err.to_string())
}
