//! Debug test for GraphQL queries

use task_master_sync::auth::GitHubAuth;

#[tokio::test]
#[ignore = "Debug test"]
async fn test_raw_graphql() {
    println!("Testing raw GraphQL query...");

    let query = r"
        query($org: String!, $number: Int!) {
            organization(login: $org) {
                projectV2(number: $number) {
                    id
                    number
                    title
                    url
                }
            }
        }
    ";

    let variables = serde_json::json!({
        "org": "5dlabs",
        "number": 9
    });

    println!("Query: {}", query);
    println!(
        "Variables: {}",
        serde_json::to_string_pretty(&variables).unwrap()
    );

    match GitHubAuth::execute_graphql(query, variables).await {
        Ok(response) => {
            println!(
                "Response: {}",
                serde_json::to_string_pretty(&response).unwrap()
            );
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}
