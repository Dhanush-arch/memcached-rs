use std::{
    collections::HashMap,
    io::{BufRead, BufReader, BufWriter, Write},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};
#[derive(Debug)]
enum Command<'a> {
    Set(SetCommand<'a>),
    Get(GetCommand<'a>),
}
#[derive(Debug)]
struct SetCommand<'a> {
    key: &'a str,
    flags: u16,
    byte_count: u128,
    no_reply: bool,
    data_block: &'a str,
}
#[derive(Debug)]
struct GetCommand<'a> {
    key: &'a str,
}

fn start_server() -> TcpListener {
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

    return listerner;
}

fn handle_connection(
    sock_stream: TcpStream,
    data_storage: Arc<Mutex<HashMap<String, (String, u16, u128)>>>,
) {
    let mut command = String::new();
    let mut buf_reader = BufReader::new(&sock_stream);
    let mut buf_writer = BufWriter::new(&sock_stream);

    loop {
        command.clear();
        match buf_reader.read_line(&mut command) {
            Ok(_) => {
                let command_vec: Vec<_> = command.trim().split_ascii_whitespace().collect();
                let processed_command: Command<'_>;
                match parse_command(command_vec) {
                    Some(p_command) => processed_command = p_command,
                    None => return,
                }
                match processed_command {
                    Command::Get(get_command) => {
                        match data_storage.lock().unwrap().get(get_command.key) {
                            Some(data) => {
                                buf_writer
                                    .write_fmt(format_args!(
                                        "VALUE {} {} {}\r\n",
                                        data.0, data.1, data.2
                                    ))
                                    .unwrap();
                                buf_writer.flush().unwrap();
                            }
                            None => {
                                buf_writer.write("END\r\n".as_bytes()).unwrap();
                                buf_writer.flush().unwrap();
                            }
                        }
                    }
                    Command::Set(mut set_command) => {
                        let mut data = String::new();
                        buf_reader.read_line(&mut data).unwrap();
                        set_command.data_block = data.as_str().trim();
                        data_storage.lock().unwrap().insert(
                            set_command.key.to_string(),
                            (
                                set_command.data_block.to_string(),
                                set_command.flags,
                                set_command.byte_count,
                            ),
                        );
                        if set_command.no_reply == false {
                            buf_writer.write_all("STORED\r\n".as_bytes()).unwrap();
                            buf_writer.flush().unwrap();
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }
}

fn parse_command(command_string: Vec<&str>) -> Option<Command> {
    match command_string.get(0) {
        Some(str) => {
            if str.eq_ignore_ascii_case("set") {
                let sc = SetCommand {
                    key: command_string.get(1).unwrap(),
                    flags: command_string.get(2).unwrap().parse::<u16>().unwrap(),
                    byte_count: command_string.get(3).unwrap().parse::<u128>().unwrap(),
                    no_reply: match command_string.get(4) {
                        Some(str) => {
                            if str.eq_ignore_ascii_case("noreply") {
                                true
                            } else {
                                false
                            }
                        }
                        None => false,
                    },
                    data_block: "",
                };
                return Some(Command::Set(sc));
            } else if str.eq_ignore_ascii_case("get") {
                let gc = GetCommand {
                    key: command_string.get(1).unwrap(),
                };
                return Some(Command::Get(gc));
            }
            return None;
        }
        None => None,
    }
}
fn main() {
    let data_storage: Arc<Mutex<HashMap<String, (String, u16, u128)>>> =
        Arc::new(Mutex::new(HashMap::new()));
    let server = start_server();

    for sock_stream in server.incoming() {
        let shared_hashmap = Arc::clone(&data_storage);
        thread::spawn(move || {
            handle_connection(sock_stream.unwrap(), shared_hashmap);
        });
    }
    // Added basic multi-threading, TODO: threadpool implementation
}
