use crate::SlippiConfig;

impl SlippiConfig {
    pub fn development() -> Self {
        Self {
            graphql_url: Some(String::from("http://localhost:3000/graphql")),
            user_rest_url: Some(String::from("http://localhost:3000/users/slp")),
            login_page_url: Some(String::from("http://localhost:3000/users/enable")),
            update_page_url: Some(String::from("http://localhost:3000/download")),
        }
    }
}
