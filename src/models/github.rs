use serde::{Deserialize, Serialize};

// Type aliases for clarity and backwards compatibility
pub type GitHubProject = Project;
pub type GitHubProjectItem = ProjectItem;
pub type GitHubField = CustomField;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum GitHubFieldType {
    Text,
    Number,
    Date,
    SingleSelect,
    Iteration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub number: i32,
    pub title: String,
    pub url: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectItem {
    pub id: String,
    pub title: String,
    pub body: Option<String>,
    #[serde(rename = "fieldValues")]
    pub field_values: Vec<FieldValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldValue {
    pub field: CustomField,
    #[serde(flatten)]
    pub value: FieldValueContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FieldValueContent {
    Text(String),
    Number(f64),
    Date(String),
    SingleSelect(String),
    Iteration(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomField {
    pub id: String,
    pub name: String,
    #[serde(rename = "dataType")]
    pub data_type: String,
    pub options: Option<Vec<FieldOption>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldOption {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
}

// GraphQL Response structures
#[derive(Debug, Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
pub struct GraphQLError {
    pub message: String,
    pub path: Option<Vec<String>>,
}
