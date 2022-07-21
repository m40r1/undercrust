use crate::{
    modules::{HttpFinding, HttpModule, Module},
    Error,
};

use async_trait::async_trait;
use reqwest::Client;

pub struct GitDirectoryDisclosure {}

impl GitDirectoryDisclosure {
    pub fn new() -> Self {
        GitDirectoryDisclosure {}
    }

    fn is_git_directory_listing(&self, content: &str) -> bool {
        return content.contains("HEAD")
            && content.contains("refs")
            && content.contains("config")
            && content.contains("index")
            && content.contains("objects");
    }
}
impl Module for GitDirectoryDisclosure {
    fn name(&self) -> String {
        String::from("http/git_directory_disclosute")
    }
    fn description(&self) -> String {
        String::from("Check for .git/ directory disclosure")
    }
}

#[async_trait]
impl HttpModule for GitDirectoryDisclosure {
    async fn scan(
        &self,
        http_client: &Client,
        endpoint: &str,
    ) -> Result<Option<HttpFinding>, Error> {
    }
}
