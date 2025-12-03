//! Table configuration types (Functional Core - pure data).

/// Table schema configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableConfig {
    pub table_name: String,
    pub partition_key: KeyAttribute,
    pub sort_key: Option<KeyAttribute>,
    pub gsis: Vec<GsiConfig>,
    pub billing_mode: BillingMode,
}

/// A key attribute definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyAttribute {
    pub name: String,
    pub attribute_type: AttributeType,
}

/// DynamoDB attribute types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeType {
    String,
}

/// Global Secondary Index configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GsiConfig {
    pub name: String,
    pub partition_key: KeyAttribute,
    pub sort_key: Option<KeyAttribute>,
    pub projection: ProjectionType,
}

/// GSI projection type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectionType {
    All,
}

/// Billing mode for the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillingMode {
    PayPerRequest,
}

impl TableConfig {
    /// Sets the table name.
    pub fn with_table_name(mut self, name: &str) -> Self {
        self.table_name = name.to_string();
        self
    }
}

/// Returns the canonical table configuration for calendsync.
/// This is a pure function - no I/O.
pub fn calendsync_table_config() -> TableConfig {
    TableConfig {
        table_name: "calendsync".to_string(),
        partition_key: KeyAttribute {
            name: "PK".to_string(),
            attribute_type: AttributeType::String,
        },
        sort_key: Some(KeyAttribute {
            name: "SK".to_string(),
            attribute_type: AttributeType::String,
        }),
        gsis: vec![GsiConfig {
            name: "GSI1".to_string(),
            partition_key: KeyAttribute {
                name: "GSI1PK".to_string(),
                attribute_type: AttributeType::String,
            },
            sort_key: Some(KeyAttribute {
                name: "GSI1SK".to_string(),
                attribute_type: AttributeType::String,
            }),
            projection: ProjectionType::All,
        }],
        billing_mode: BillingMode::PayPerRequest,
    }
}
