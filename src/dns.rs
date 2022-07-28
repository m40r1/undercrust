use crate::modules::Subdomain;
use std::{sync::Arc, time::Duration};
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::name_server::GenericConnection;
use trust_dns_resolver::name_server::GenericConnectionProvider;
use trust_dns_resolver::name_server::TokioRuntime;
use trust_dns_resolver::AsyncResolver;
pub type Resolver = Arc<AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>>>;

//does a lookup
pub async fn resolves(dns_resolver: &Resolver, domain: Subdomain) -> Option<Subdomain> {
    if dns_resolver.lookup_ip(domain.domain.as_str()).await.is_ok() {
        return Some(domain);
    }
    None
}
//creates an resolver
// this has to be DOH to be secure/anon
pub fn new_resolver() -> Resolver {
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(4);

    let mut resolver = AsyncResolver::tokio(ResolverConfig::quad9(), opts)
        .expect("dns/new_resolver: building DNS client");
    return Arc::new(resolver);
}
