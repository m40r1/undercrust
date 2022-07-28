use crate::{
    common_ports::MOST_COMMON_PORTS,
    modules::{Port, Subdomain},
};

use futures::{stream, StreamExt};
use std::{
    net::{SocketAddr, ToSocketAddrs},
    time::Duration,
};
use tokio::net::TcpStream;

//?????
pub async fn scan_ports(concur: usize, mut subdomain: Subdomain) -> Subdomain {
    //puts the subdomain in the socket_addr
    let socket_addr: Vec<SocketAddr> = format!("{}:1024", subdomain.domain)
        .to_socket_addrs()
        .expect("port scanner:creating sock addr")
        .collect();

    if socket_addr.len() == 0 {
        return subdomain;
    }

    let socket_adr = socket_addr[0];
    //tests subdomain ports
    //it tries the list of all the common ports per subdomain
    subdomain.open_ports = stream::iter(MOST_COMMON_PORTS.into_iter())
        .map(|port| async move {
            let port = scan_port(socket_adr, *port).await;
            if port.is_open {
                return Some(port);
            }
            None
        })
        .buffer_unordered(concur)
        .filter_map(|port| async { port })
        .collect()
        .await;
    subdomain
}
//creates a stream to the socket addr
//if it connects return ok and create a vec for the findings
//later used for vuln scan
async fn scan_port(mut socket_addr: SocketAddr, port: u16) -> Port {
    let timeout = Duration::from_secs(3); //3 secs its closed
    socket_addr.set_port(port);
    let is_open = matches!(
        tokio::time::timeout(timeout, TcpStream::connect(&socket_addr)).await,
        Ok(Ok(_)),
    );
    Port {
        port,
        is_open,
        findings: Vec::new(),
    }
}
