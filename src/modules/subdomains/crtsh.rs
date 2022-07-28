use crate::{
    modules::{Module, SubdomainModule},
    Error,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
// Crtsh is the site were querying for domain info
pub struct Crtsh {}

impl Crtsh {
    pub fn new() -> Self {
        Crtsh {}
    }
}

impl Module for Crtsh {
    fn name(&self) -> String {
        String::from("subdomains/crtsh")
    }
    fn description(&self) -> String {
        String::from("use crt.sh/ to find subdomains")
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct CrtShEntry {
    name_value: String,
}

#[async_trait]
//this is all used in cli
impl SubdomainModule for Crtsh {
    async fn enumerate(&self, domain: &str) -> Result<Vec<String>, Error> {
        //the query
        // returns as json
        let url = format!("https://crt.sh/?q=%25.{}&output=json", domain);
        //makes the query
        let res = reqwest::get(&url).await?;
        //if it fails returns err
        if !res.status().is_success() {
            return Err(Error::InvalidHttpResponse(self.name()));
        }
        //gets all the domains entries from crtsh
        let crtsh_entries: Vec<CrtShEntry> = match res.json().await {
            Ok(info) => info,
            Err(_) => return Err(Error::InvalidHttpResponse(self.name())),
        };
        // clean entries to a list of subdomains
        let subdomains: HashSet<String> = crtsh_entries
            .into_iter()
            .map(|entry| {
                entry
                    .name_value
                    .split("\n")
                    .map(|subdomain| subdomain.trim().to_string())
                    .collect::<Vec<String>>()
            })
            .flatten()
            .filter(|subdomain: &String| !subdomain.contains("*"))
            .collect();
        Ok(subdomains.into_iter().collect())
    }
}
