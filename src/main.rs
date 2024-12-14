use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener};
fn main() {
    let mut port = 11211;
    let args: Vec<String> = std::env::args().collect();

    for (i, arg) in args.iter().enumerate() {
        if arg.eq_ignore_ascii_case("-p") && i < args.len() - 1 {
            port = args[i + 1]
                .trim()
                .parse::<u16>()
                .unwrap_or_else(|_| panic!("{} is not a proper port number.", args[i + 1].trim()));
            break;
        }
    }

    let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port));

    let listerner = TcpListener::bind(addr).unwrap();
    match listerner.accept() {
        Ok((_sockter, socketAddr)) => println!("Connected: {}", _sockter.local_addr().unwrap()),
        Err(_) => println!("Couldn't connect"),
    }
}
