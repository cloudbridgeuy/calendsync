//! Table deployment operations (Imperative Shell).

use super::client;
use super::config::{self, TableConfig};
use super::error::{DynamodbError, Result};
use super::planning::{DeployPlan, DestroyPlan, GsiStatus, TableStatus};
use aws_sdk_dynamodb::types::{
    AttributeDefinition, BillingMode, GlobalSecondaryIndex, KeySchemaElement, KeyType, Projection,
    ProjectionType, ScalarAttributeType,
};
use aws_sdk_dynamodb::Client;
use std::time::Duration;

/// Execute a deploy plan.
pub async fn execute_deploy_plan(client: &Client, plan: &DeployPlan) -> Result<()> {
    match plan {
        DeployPlan::CreateTable { config } => {
            create_table(client, config).await?;
            wait_for_table_active(client, &config.table_name).await?;
        }
        DeployPlan::AddGsis {
            table_name,
            gsis_to_add,
        } => {
            for gsi in gsis_to_add {
                add_gsi(client, table_name, gsi).await?;
                wait_for_table_active(client, table_name).await?;
            }
        }
        DeployPlan::NoChanges { .. } => {
            // Nothing to do
        }
    }
    Ok(())
}

/// Execute a destroy plan.
pub async fn execute_destroy_plan(client: &Client, plan: &DestroyPlan) -> Result<()> {
    match plan {
        DestroyPlan::DeleteTable { table_name } => {
            delete_table(client, table_name).await?;
        }
        DestroyPlan::AlreadyGone { .. } => {
            // Nothing to do
        }
    }
    Ok(())
}

async fn create_table(client: &Client, config: &TableConfig) -> Result<()> {
    let mut key_schema = vec![KeySchemaElement::builder()
        .attribute_name(&config.partition_key.name)
        .key_type(KeyType::Hash)
        .build()
        .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?];

    let mut attribute_definitions = vec![AttributeDefinition::builder()
        .attribute_name(&config.partition_key.name)
        .attribute_type(to_scalar_type(&config.partition_key.attribute_type))
        .build()
        .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?];

    if let Some(sk) = &config.sort_key {
        key_schema.push(
            KeySchemaElement::builder()
                .attribute_name(&sk.name)
                .key_type(KeyType::Range)
                .build()
                .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?,
        );
        attribute_definitions.push(
            AttributeDefinition::builder()
                .attribute_name(&sk.name)
                .attribute_type(to_scalar_type(&sk.attribute_type))
                .build()
                .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?,
        );
    }

    // Add GSI attribute definitions
    for gsi in &config.gsis {
        // Add partition key if not already defined
        let pk_name = gsi.partition_key.name.as_str();
        if !attribute_definitions
            .iter()
            .any(|a| a.attribute_name() == pk_name)
        {
            attribute_definitions.push(
                AttributeDefinition::builder()
                    .attribute_name(&gsi.partition_key.name)
                    .attribute_type(to_scalar_type(&gsi.partition_key.attribute_type))
                    .build()
                    .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?,
            );
        }

        // Add sort key if not already defined
        if let Some(sk) = &gsi.sort_key {
            let sk_name = sk.name.as_str();
            if !attribute_definitions
                .iter()
                .any(|a| a.attribute_name() == sk_name)
            {
                attribute_definitions.push(
                    AttributeDefinition::builder()
                        .attribute_name(&sk.name)
                        .attribute_type(to_scalar_type(&sk.attribute_type))
                        .build()
                        .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?,
                );
            }
        }
    }

    let mut request = client
        .create_table()
        .table_name(&config.table_name)
        .set_key_schema(Some(key_schema))
        .set_attribute_definitions(Some(attribute_definitions))
        .billing_mode(BillingMode::PayPerRequest);

    // Add GSIs
    for gsi in &config.gsis {
        let mut gsi_key_schema = vec![KeySchemaElement::builder()
            .attribute_name(&gsi.partition_key.name)
            .key_type(KeyType::Hash)
            .build()
            .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?];

        if let Some(sk) = &gsi.sort_key {
            gsi_key_schema.push(
                KeySchemaElement::builder()
                    .attribute_name(&sk.name)
                    .key_type(KeyType::Range)
                    .build()
                    .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?,
            );
        }

        request = request.global_secondary_indexes(
            GlobalSecondaryIndex::builder()
                .index_name(&gsi.name)
                .set_key_schema(Some(gsi_key_schema))
                .projection(
                    Projection::builder()
                        .projection_type(ProjectionType::All)
                        .build(),
                )
                .build()
                .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?,
        );
    }

    request
        .send()
        .await
        .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?;
    Ok(())
}

async fn add_gsi(client: &Client, table_name: &str, gsi: &config::GsiConfig) -> Result<()> {
    use aws_sdk_dynamodb::types::{CreateGlobalSecondaryIndexAction, GlobalSecondaryIndexUpdate};

    let mut gsi_key_schema = vec![KeySchemaElement::builder()
        .attribute_name(&gsi.partition_key.name)
        .key_type(KeyType::Hash)
        .build()
        .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?];

    if let Some(sk) = &gsi.sort_key {
        gsi_key_schema.push(
            KeySchemaElement::builder()
                .attribute_name(&sk.name)
                .key_type(KeyType::Range)
                .build()
                .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?,
        );
    }

    // Build attribute definitions for the GSI keys
    let mut attribute_definitions = vec![AttributeDefinition::builder()
        .attribute_name(&gsi.partition_key.name)
        .attribute_type(to_scalar_type(&gsi.partition_key.attribute_type))
        .build()
        .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?];

    if let Some(sk) = &gsi.sort_key {
        attribute_definitions.push(
            AttributeDefinition::builder()
                .attribute_name(&sk.name)
                .attribute_type(to_scalar_type(&sk.attribute_type))
                .build()
                .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?,
        );
    }

    client
        .update_table()
        .table_name(table_name)
        .set_attribute_definitions(Some(attribute_definitions))
        .global_secondary_index_updates(
            GlobalSecondaryIndexUpdate::builder()
                .create(
                    CreateGlobalSecondaryIndexAction::builder()
                        .index_name(&gsi.name)
                        .set_key_schema(Some(gsi_key_schema))
                        .projection(
                            Projection::builder()
                                .projection_type(ProjectionType::All)
                                .build(),
                        )
                        .build()
                        .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?,
                )
                .build(),
        )
        .send()
        .await
        .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?;

    Ok(())
}

async fn delete_table(client: &Client, table_name: &str) -> Result<()> {
    client
        .delete_table()
        .table_name(table_name)
        .send()
        .await
        .map_err(|e| DynamodbError::AwsSdk(e.to_string()))?;
    Ok(())
}

async fn wait_for_table_active(client: &Client, table_name: &str) -> Result<()> {
    let max_attempts = 60;
    let delay = Duration::from_secs(2);

    for _ in 0..max_attempts {
        if let Some(state) = client::get_table_state(client, table_name).await? {
            if state.status == TableStatus::Active {
                // Also check all GSIs are active
                let all_gsis_active = state.gsis.iter().all(|g| g.status == GsiStatus::Active);
                if all_gsis_active {
                    return Ok(());
                }
            }
        }
        tokio::time::sleep(delay).await;
    }

    Err(DynamodbError::TableActivationTimeout)
}

fn to_scalar_type(attr_type: &config::AttributeType) -> ScalarAttributeType {
    match attr_type {
        config::AttributeType::String => ScalarAttributeType::S,
    }
}
