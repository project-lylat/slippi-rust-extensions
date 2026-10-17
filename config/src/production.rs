use crate::SlippiConfig;

impl SlippiConfig {
    pub fn production() -> Self {
        Self {
            graphql_url: Some(String::from("https://lylat.gg/graphql")),
            user_rest_url: Some(String::from("https://lylat.gg/users/slp")),
            login_page_url: Some(String::from("https://lylat.gg/users/enable")),
            update_page_url: Some(String::from("https://lylat.gg/download")),
            account_file: Some(String::from("lylat.json")),
        }
    }
}
