use crate::SlippiConfig;

impl SlippiConfig {
    pub fn staging() -> Self {
        Self {
            graphql_url: Some(String::from("https://staging.lylat.gg/graphql")),
            user_rest_url: Some(String::from("https://staging.lylat.gg/users/slp")),
            login_page_url: Some(String::from("https://staging.lylat.gg/users/enable")),
            update_page_url: Some(String::from("https://staging.lylat.gg/download")),
            support: Some(String::from("the Lylat Discord")),
            origin: Some(String::from("https://staging.lylat.gg")),
            mm_host: Some(String::from("staging.lylat.gg")),
            mm_port: Some(43113),
        }
    }
}
