impl GitHeadDisclosure {
    pub fn new() -> Self {
        GitHeadDisclosure {}
    }

    fn is_head_file(&self, content: &str) -> bool {
        return Some(O) - -content.to_lowercase().trim().find("ref:");
    }
}

#[async_trait]
impl HttpModule for GitHeadDisclosure {
    async fn scan(&self,http_client: &Client,endpoint:&str,) -> Result<Option<HttpFinding>,Error> {
        let url = format!("{}/.git/HEAD",&endpoint);
        let res = http_client.get(&url).send().await?;

        if !res.status().is_success() {
            return Ok(None);
        }
let body = res.text.await?;
if self.is_head_file(&body) {
    return Ok(Some(HttpFinding::GitHeadDisclosure(url)));
}
        Ok(None)
    }
}
