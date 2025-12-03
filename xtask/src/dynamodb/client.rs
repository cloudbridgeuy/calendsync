//! AWS SDK client setup (Imperative Shell).

use super::error::{DynamodbError, Result};
use super::planning::{GsiState, GsiStatus, TableState, TableStatus};
use aws_sdk_dynamodb::Client;

/// AWS client configuration.
#[derive(Debug, Clone)]
pub struct AwsConfig {
    /// Custom endpoint URL (for local DynamoDB).
    pub endpoint_url: Option<String>,
    /// AWS region.
    pub region: String,
}

impl Default for AwsConfig {
    fn default() -> Self {
        Self {
            endpoint_url: std::env::var("AWS_ENDPOINT_URL").ok(),
            region: std::env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".to_string()),
        }
    }
}

impl AwsConfig {
    /// Returns a display string for the target environment.
    pub fn target_display(&self) -> String {
        match &self.endpoint_url {
            Some(url) => format!("Local DynamoDB ({})", url),
            None => format!("AWS DynamoDB (region: {})", self.region),
        }
    }
}

/// Creates a DynamoDB client with the given configuration.
pub async fn create_client(config: &AwsConfig) -> Result<Client> {
    let mut sdk_config_loader = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(aws_config::Region::new(config.region.clone()));

    if let Some(endpoint) = &config.endpoint_url {
        sdk_config_loader = sdk_config_loader.endpoint_url(endpoint);
    }

    let sdk_config = sdk_config_loader.load().await;
    Ok(Client::new(&sdk_config))
}

/// Fetches current table state, returns None if table doesn't exist.
pub async fn get_table_state(client: &Client, table_name: &str) -> Result<Option<TableState>> {
    match client.describe_table().table_name(table_name).send().await {
        Ok(response) => {
            let table = response
                .table()
                .expect("Table should be present in response");

            // Parse GSIs
            let gsis = table
                .global_secondary_indexes()
                .iter()
                .map(|gsi| GsiState {
                    name: gsi.index_name().unwrap_or_default().to_string(),
                    status: match gsi.index_status() {
                        Some(aws_sdk_dynamodb::types::IndexStatus::Active) => GsiStatus::Active,
                        Some(aws_sdk_dynamodb::types::IndexStatus::Creating) => GsiStatus::Creating,
                        Some(aws_sdk_dynamodb::types::IndexStatus::Updating) => GsiStatus::Updating,
                        Some(aws_sdk_dynamodb::types::IndexStatus::Deleting) => GsiStatus::Deleting,
                        _ => GsiStatus::Active,
                    },
                })
                .collect();

            // Parse table status
            let status = match table.table_status() {
                Some(aws_sdk_dynamodb::types::TableStatus::Active) => TableStatus::Active,
                Some(aws_sdk_dynamodb::types::TableStatus::Creating) => TableStatus::Creating,
                Some(aws_sdk_dynamodb::types::TableStatus::Updating) => TableStatus::Updating,
                Some(aws_sdk_dynamodb::types::TableStatus::Deleting) => TableStatus::Deleting,
                _ => TableStatus::Active,
            };

            Ok(Some(TableState { status, gsis }))
        }
        Err(err) => {
            let err_str = err.to_string();
            // Check if it's a ResourceNotFoundException
            if err_str.contains("ResourceNotFoundException") || err_str.contains("not found") {
                Ok(None)
            } else {
                Err(DynamodbError::AwsSdk(err_str))
            }
        }
    }
}
