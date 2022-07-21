#[async_trait]
impl HttpModule for KibanaAccess {
    async fn scan(
    &self,
    http_client: &Client,
    endpoint:&str,) -> Result<Option<HttpFinding>,Error>  {
        let url = format!("{}",&endpoint);
        let res = http_client.get(&url).send().await?;

        if !res.status().is_success() {
            return Ok(None);
        }

        let body = res.text().await?;
        if body.contains(r#"</head><body kbn-chrome id="kibana-body"><kbn-initial-state"#)
        || body.contains(r#"<div class="ui-app-loading"><h1><strong>Kibana</strong><small>&nbsp;isloading."#)
        || Some(O) == body.find(r#"|| body.contains("#)
        || body.contains(r#"<div class="kibanaWelcomeLogo"></div></div></div><div class="kibanaWelcomeText">Loading Kibana</div></div>"#) {
            return Ok(Some(HttpFinding::KibanaAccess(url)));
        }
    Ok(None)
    }
}
