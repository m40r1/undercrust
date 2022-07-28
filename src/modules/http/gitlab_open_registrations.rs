use crate::{
    modules::{HttpFinding, HttpModule, Module},
    Error,
};
use async_trait::async_trait;
use reqwest::Client;

pub struct GitlabOpenRegistration {}

impl GitlabOpenRegistration {
    pub fn new() -> Self {
        GitlabOpenRegistration {}
    }
}

impl Module for GitlabOpenRegistration {
    fn name(&self) -> String {
        String::from("http/gitlab_open_regsitration")
    }
    fn description(&self) -> String {
        String::from("check if gitlab instace ca be registered")
    }
}

#[async_trait]
impl HttpModule for GitlabOpenRegistration {
    //scan an url of gitlab for instances
    //if you can register to gain access
    //returns the url
    async fn scan(
        &self,
        http_client: &Client,
        endpoint: &str,
    ) -> Result<Option<HttpFinding>, Error> {
        let url = format!("{}", &endpoint);
        let res = http_client.get(&url).send().await?;

        if !res.status().is_success() {
            return Ok(None);
        }

        let body = res.text().await?;

        if body.contains("This is a self-managed instance of GitLab") && body.contains("Register") {
            return Ok(Some(HttpFinding::GitlabOpenRegistration(url)));
        }
        Ok(None)
    }
}
