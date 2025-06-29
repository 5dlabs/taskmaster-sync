# GitHub Projects v2 API Reference

## Overview

This document provides a comprehensive reference for the GitHub Projects v2 GraphQL API, specifically focused on the operations needed for Task Master Sync.

## Official Resources

### Complete API Documentation
- **GitHub GraphQL API Docs**: https://docs.github.com/en/graphql
- **Public Schema Download**: https://docs.github.com/public/schema.docs.graphql
- **GraphQL Explorer**: https://docs.github.com/en/graphql/overview/explorer
- **Objects Reference**: https://docs.github.com/en/graphql/reference/objects

### Schema Files Available
- `github-schema-introspection.json` - Complete introspection result (2.9MB)
- `github-projects-v2-schema.json` - ProjectV2 specific types
- `github-api-schema.json` - Basic schema info

## Key ProjectV2 Types

### Core Objects

#### ProjectV2
The main project object containing items, fields, and metadata.

```graphql
type ProjectV2 implements Closable & Node & Updatable {
  id: ID!
  title: String!
  shortDescription: String
  readme: String
  url: URI!
  number: Int!
  public: Boolean!
  closed: Boolean!
  closedAt: DateTime
  createdAt: DateTime!
  updatedAt: DateTime!
  deletedAt: DateTime
  creator: Actor
  owner: ProjectV2Owner!

  # Items and pagination
  items(
    after: String
    before: String
    first: Int
    last: Int
    orderBy: ProjectV2ItemOrder
  ): ProjectV2ItemConnection!

  # Fields management
  fields(
    after: String
    before: String
    first: Int
    last: Int
    orderBy: ProjectV2FieldOrder
  ): ProjectV2FieldConnection!

  # Views
  views(
    after: String
    before: String
    first: Int
    last: Int
    orderBy: ProjectV2ViewOrder
  ): ProjectV2ViewConnection!
}
```

#### ProjectV2Item
Individual items within a project (tasks, issues, pull requests).

```graphql
type ProjectV2Item implements Node {
  id: ID!
  project: ProjectV2!
  createdAt: DateTime!
  updatedAt: DateTime!
  archived: Boolean!
  isArchived: Boolean!

  # The actual content (issue, PR, or draft issue)
  content: ProjectV2ItemContent

  # Field values for this item
  fieldValues(
    after: String
    before: String
    first: Int
    last: Int
  ): ProjectV2ItemFieldValueConnection!

  # Field value by field ID
  fieldValueByName(name: String!): ProjectV2ItemFieldValue
}
```

#### ProjectV2Field
Custom fields within a project.

```graphql
interface ProjectV2Field {
  id: ID!
  project: ProjectV2!
  name: String!
  createdAt: DateTime!
  updatedAt: DateTime!
}

# Specific field types
type ProjectV2SingleSelectField implements ProjectV2Field {
  options: [ProjectV2SingleSelectFieldOption!]!
}

type ProjectV2IterationField implements ProjectV2Field {
  configuration: ProjectV2IterationFieldConfiguration!
}
```

## Essential Mutations

### 1. Create Project Items

#### addProjectV2DraftIssue
Creates a new draft issue in a project.

```graphql
mutation addProjectV2DraftIssue($input: AddProjectV2DraftIssueInput!) {
  addProjectV2DraftIssue(input: $input) {
    projectItem {
      id
      content {
        ... on DraftIssue {
          id
          title
          body
        }
      }
    }
  }
}
```

**Input:**
```graphql
input AddProjectV2DraftIssueInput {
  projectId: ID!
  title: String!
  body: String
  assigneeIds: [ID!]
  clientMutationId: String
}
```

### 2. Update Project Items

#### updateProjectV2DraftIssue
Updates an existing draft issue.

```graphql
mutation updateProjectV2DraftIssue($input: UpdateProjectV2DraftIssueInput!) {
  updateProjectV2DraftIssue(input: $input) {
    draftIssue {
      id
      title
      body
    }
  }
}
```

**Input:**
```graphql
input UpdateProjectV2DraftIssueInput {
  draftIssueId: ID!
  title: String
  body: String
  assigneeIds: [ID!]
  clientMutationId: String
}
```

### 3. Update Field Values

#### updateProjectV2ItemFieldValue
Sets or updates a field value for a project item.

```graphql
mutation updateProjectV2ItemFieldValue($input: UpdateProjectV2ItemFieldValueInput!) {
  updateProjectV2ItemFieldValue(input: $input) {
    projectV2Item {
      id
      fieldValueByName(name: "Status") {
        ... on ProjectV2ItemFieldSingleSelectValue {
          name
          optionId
        }
      }
    }
  }
}
```

**Input:**
```graphql
input UpdateProjectV2ItemFieldValueInput {
  projectId: ID!
  itemId: ID!
  fieldId: ID!
  value: ProjectV2FieldValue!
  clientMutationId: String
}
```

### 4. Create Custom Fields

#### createProjectV2Field
Creates a new custom field in a project.

```graphql
mutation createProjectV2Field($input: CreateProjectV2FieldInput!) {
  createProjectV2Field(input: $input) {
    projectV2Field {
      ... on ProjectV2SingleSelectField {
        id
        name
        options {
          id
          name
        }
      }
      ... on ProjectV2Field {
        id
        name
      }
    }
  }
}
```

**Input:**
```graphql
input CreateProjectV2FieldInput {
  projectId: ID!
  dataType: ProjectV2FieldType!
  name: String!
  singleSelectOptions: [ProjectV2SingleSelectFieldOptionInput!]
  clientMutationId: String
}
```

## Essential Queries

### 1. Get Project Information

```graphql
query getProject($owner: String!, $number: Int!) {
  organization(login: $owner) {
    projectV2(number: $number) {
      id
      title
      shortDescription
      url
      number
      public
      closed
      createdAt
      updatedAt

      fields(first: 100) {
        nodes {
          ... on ProjectV2Field {
            id
            name
            createdAt
          }
          ... on ProjectV2SingleSelectField {
            options {
              id
              name
            }
          }
        }
      }
    }
  }
}
```

### 2. Get Project Items

```graphql
query getProjectItems($projectId: ID!, $after: String) {
  node(id: $projectId) {
    ... on ProjectV2 {
      items(first: 100, after: $after) {
        pageInfo {
          hasNextPage
          endCursor
        }
        nodes {
          id
          createdAt
          updatedAt
          archived

          content {
            ... on DraftIssue {
              id
              title
              body
              createdAt
              updatedAt
            }
            ... on Issue {
              id
              title
              body
              number
              state
              createdAt
              updatedAt
            }
            ... on PullRequest {
              id
              title
              body
              number
              state
              createdAt
              updatedAt
            }
          }

          fieldValues(first: 50) {
            nodes {
              ... on ProjectV2ItemFieldTextValue {
                text
                field {
                  ... on ProjectV2Field {
                    id
                    name
                  }
                }
              }
              ... on ProjectV2ItemFieldSingleSelectValue {
                name
                optionId
                field {
                  ... on ProjectV2SingleSelectField {
                    id
                    name
                  }
                }
              }
            }
          }
        }
      }
    }
  }
}
```

### 3. Get Specific Item by Custom Field

```graphql
query findItemByTmId($projectId: ID!, $tmId: String!) {
  node(id: $projectId) {
    ... on ProjectV2 {
      items(first: 100) {
        nodes {
          id
          fieldValues(first: 50) {
            nodes {
              ... on ProjectV2ItemFieldTextValue {
                text
                field {
                  ... on ProjectV2Field {
                    id
                    name
                  }
                }
              }
            }
          }
          content {
            ... on DraftIssue {
              id
              title
              body
            }
          }
        }
      }
    }
  }
}
```

## Field Types and Values

### ProjectV2FieldType Enum
```graphql
enum ProjectV2FieldType {
  ASSIGNEES
  DATE
  ITERATION
  LINKED_PULL_REQUESTS
  MILESTONE
  NUMBER
  REPOSITORY
  REVIEWERS
  SINGLE_SELECT
  TEXT
  TITLE
  TRACKED_BY
  TRACKS
}
```

### Field Value Types

#### Text Field
```graphql
type ProjectV2ItemFieldTextValue {
  text: String
  field: ProjectV2Field!
}
```

#### Single Select Field
```graphql
type ProjectV2ItemFieldSingleSelectValue {
  name: String
  optionId: String
  field: ProjectV2SingleSelectField!
}
```

#### Number Field
```graphql
type ProjectV2ItemFieldNumberValue {
  number: Float
  field: ProjectV2Field!
}
```

## Task Master Sync Specific Usage

### Required Custom Fields

1. **TM_ID** (Text) - Maps Taskmaster task IDs
2. **Dependencies** (Text) - Comma-separated task dependencies
3. **Test Strategy** (Text) - Testing approach for the task

### Field Creation Example

```javascript
// Create TM_ID field
const createTmIdField = `
  mutation createTmIdField($projectId: ID!) {
    createProjectV2Field(input: {
      projectId: $projectId
      dataType: TEXT
      name: "TM_ID"
    }) {
      projectV2Field {
        ... on ProjectV2Field {
          id
          name
        }
      }
    }
  }
`;

// Create Dependencies field
const createDependenciesField = `
  mutation createDependenciesField($projectId: ID!) {
    createProjectV2Field(input: {
      projectId: $projectId
      dataType: TEXT
      name: "Dependencies"
    }) {
      projectV2Field {
        ... on ProjectV2Field {
          id
          name
        }
      }
    }
  }
`;

// Create Test Strategy field
const createTestStrategyField = `
  mutation createTestStrategyField($projectId: ID!) {
    createProjectV2Field(input: {
      projectId: $projectId
      dataType: TEXT
      name: "Test Strategy"
    }) {
      projectV2Field {
        ... on ProjectV2Field {
          id
          name
        }
      }
    }
  }
`;
```

### Sync Operation Examples

#### Create Task Item
```javascript
const createTaskItem = `
  mutation createTaskItem($projectId: ID!, $title: String!, $body: String!) {
    addProjectV2DraftIssue(input: {
      projectId: $projectId
      title: $title
      body: $body
    }) {
      projectItem {
        id
        content {
          ... on DraftIssue {
            id
            title
            body
          }
        }
      }
    }
  }
`;
```

#### Set TM_ID Field
```javascript
const setTmIdField = `
  mutation setTmId($projectId: ID!, $itemId: ID!, $fieldId: ID!, $tmId: String!) {
    updateProjectV2ItemFieldValue(input: {
      projectId: $projectId
      itemId: $itemId
      fieldId: $fieldId
      value: {
        text: $tmId
      }
    }) {
      projectV2Item {
        id
      }
    }
  }
`;
```

## Rate Limiting

- **GraphQL API**: 5,000 points per hour
- **Node limit**: 500,000 nodes per query
- **Time limit**: 10 seconds per query

### Best Practices
- Use pagination for large datasets
- Implement exponential backoff
- Cache project structure (fields, etc.)
- Batch operations where possible

## Error Handling

### Common Error Types
- `NOT_FOUND` - Project or item doesn't exist
- `FORBIDDEN` - Insufficient permissions
- `UNPROCESSABLE` - Invalid input data
- `RATE_LIMITED` - API rate limit exceeded

### Example Error Response
```json
{
  "errors": [
    {
      "type": "NOT_FOUND",
      "path": ["organization", "projectV2"],
      "message": "Could not resolve to a ProjectV2 with the number 999."
    }
  ]
}
```

## Authentication

Uses GitHub CLI authentication:
```bash
gh auth status
gh api graphql --field query='...' --field variables='...'
```

## Useful Resources

- **GraphQL Explorer**: https://docs.github.com/en/graphql/overview/explorer
- **Projects v2 Docs**: https://docs.github.com/en/issues/planning-and-tracking-with-projects
- **API Rate Limits**: https://docs.github.com/en/graphql/overview/rate-limits-and-node-limits
- **Schema Downloads**: Available in this project directory

---

*This reference covers the essential operations for Task Master Sync. For complete API documentation, refer to the official GitHub GraphQL API documentation.*