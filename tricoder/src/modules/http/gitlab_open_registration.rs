#[async_trait]
impl HttpModule for GitlabOpenRegistation {
//scan an url of gitlab for instances
//if you can register to gain access
//returns the url
    async fn scan(
    &self,
    http_client: &Client,
    endpoint: &str,) -> Result<Option<HttpFinding>,Error> {
        let url = format!("{}",&endpoint);
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
