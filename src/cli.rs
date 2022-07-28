use futures::{stream, StreamExt};
use reqwest::{Client, Proxy};
use std::{
    collections::HashSet,
    iter::FromIterator,
    time::{Duration, Instant},
};

use crate::dns;
use crate::ports;
use crate::{modules, modules::HttpModule, modules::Subdomain, Error};
// calls all modules
pub fn modules() {
    let http_modules = modules::all_http_modules();
    let subdomain_modules = modules::all_subdomains_modules();

    println!("subdomain modules");
    for module in subdomain_modules {
        println!("  {}:{}", module.name(), module.description());
    }
}

pub fn scan(target: &str) -> Result<(), Error> {
    log::info!("scanning:{}", target);
    //creates runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("building tokio runtime");
    //sets response time
    let http_timeout = Duration::from_secs(10);
    // builds client
    //tor is good but comment this if you dont get shit
    //you never know when its tors fault for beign blocked
    // let proxy = reqwest::Proxy::all("socks5h://127.0.0.1:9050").expect("no tor?");
    let http_client = Client::builder()
        .timeout(http_timeout)
        .danger_accept_invalid_certs(true)
        // .user_agent("")
        // .proxy(proxy)
        .build()?;

    // builds dns
    let dns_resolver = dns::new_resolver();
    // sets limits for concur
    let subdomains_concur = 20;
    let dns_concur = 100;
    let ports_concur = 200;
    let vuln_concur = 20;
    //trigger to start
    let scan_start = Instant::now();
    //get all subdomain related code
    let subdomains_modules = modules::all_subdomains_modules();
    //concur
    runtime.block_on(async move {
        // uses modules to enumarete subdomains
        // ma
        let mut subdomains: Vec<String> = stream::iter(subdomains_modules.into_iter())
            .map(|module| async move {
                match module.enumerate(target).await {
                    Ok(new_subdomains) => Some(new_subdomains),
                    Err(err) => {
                        log::error!("subdomains/{}: {}", module.name(), err);
                        None
                    }
                }
            })
            .buffer_unordered(subdomains_concur)
            .filter_map(|domain| async { domain })
            .collect::<Vec<Vec<String>>>()
            .await
            .into_iter()
            .flatten()
            .collect();
        // ??maybe this puts the domain in the subdomains
        subdomains.push(target.to_string());
        // maps the domain to a Subdomain struct
        // now we can deal with the ports
        let subdomains: Vec<Subdomain> = HashSet::<String>::from_iter(subdomains.into_iter())
            .into_iter()
            .filter(|subdomain| subdomain.contains(target))
            .map(|domain| Subdomain {
                domain,
                open_ports: Vec::new(),
            })
            .collect();
        log::info!("Found {} domains", subdomains.len());
        // do a dns lookup on the domains
        // it serves as a check?
        let subdomains: Vec<Subdomain> = stream::iter(subdomains.into_iter())
            .map(|domain| dns::resolves(&dns_resolver, domain))
            .buffer_unordered(dns_concur)
            .filter_map(|domain| async move { domain })
            .collect()
            .await;
        //gets the open ports
        for subdomain in &subdomains {
            println!("{}", subdomain.domain);
            for port in &subdomain.open_ports {
                println!("  {}", port.port);
            }
        }
        println!("-----------------vuln---------------------");
        let mut targets: Vec<(Box<dyn HttpModule>, String)> = Vec::new();
        // tranforms subdomains/ports in a target & link it with an http module for exploitation
        for subdomain in subdomains {
            for port in subdomain.open_ports {
                let http_modules = modules::all_http_modules();
                for http_module in http_modules {
                    let target = format!("http://{}:{:?}", &subdomain.domain, port);
                    targets.push((http_module, target));
                }
            }
        }

        // uses http modules to scan for vulns
        // iter the targets testing the exploit modules
        stream::iter(targets.into_iter())
            .for_each_concurrent(vuln_concur, |(module, target)| {
                let http_client = http_client.clone();
                async move {
                    match module.scan(&http_client, &target).await {
                        Ok(Some(finding)) => println!("{:?}", &finding),
                        Ok(None) => {}
                        Err(err) => log::debug!("Error: {}", err),
                    };
                }
            })
            .await;
    });
    //how long it took
    let scan_duration = scan_start.elapsed();
    log::info!("scan completed in {:?}", scan_duration);
    Ok(())
}
