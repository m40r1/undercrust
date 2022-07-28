use crate::{
    modules::{Module, SubdomainModule},
    Error,
};
use async_trait::async_trait;
use regex::Regex;

pub struct Dnsrepo {}

impl Dnsrepo {
    pub fn new() -> Self {
        Dnsrepo {}
    }
}

impl Module for Dnsrepo {
    fn name(&self) -> String {
        String::from("subdomains/dsrepo")
    }
    fn description(&self) -> String {
        String::from("checks dnsrepo for subdomain")
    }
}

#[async_trait]
impl SubdomainModule for Dnsrepo {
    async fn enumerate(&self, domain: &str) -> Result<Vec<String>, Error> {
        let url = format!("https://dnsrepo.noc.org/?domain={}", domain);

        let res = reqwest::get(&url).await?;
        if !res.status().is_success() {
            return Err(Error::InvalidHttpResponse(self.name()));
        }

        let mut subdomain_regex_str = r"([[:alnum:]]*?)\.".to_string();
        subdomain_regex_str.push_str(domain);
        //this shouldnt fail?
        let regex = Regex::new(&subdomain_regex_str).unwrap();
        let mut subdomain_list: Vec<String> = Vec::new();
        for subdomain in regex.captures_iter(&res.text().await?) {
            subdomain_list.push(subdomain[0].to_string());
        }
        subdomain_list.sort();
        subdomain_list.dedup();
        Ok(subdomain_list.into_iter().collect())
    }
}
