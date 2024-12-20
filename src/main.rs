use std::{
    collections::HashMap,
    io::{BufRead, BufReader, BufWriter, Write},
    net::{Ipv4Addr, SocketAddr, SocketAddrV4, TcpListener, TcpStream},
    sync::{Arc, Mutex},
    time::SystemTime,
};

pub mod threads;

const THREAD_COUNT: usize = 2;

#[derive(Debug)]
enum Command<'a> {
    Set(SetAddCommand<'a>),
    Get(GetCommand<'a>),
    Add(SetAddCommand<'a>),
    Replace(SetAddCommand<'a>),
}
#[derive(Debug)]
struct SetAddCommand<'a> {
    key: &'a str,
    flags: u16,
    byte_count: u128,
    expiry_date: i128,
    no_reply: bool,
    data_block: &'a str,
}
#[derive(Debug)]
struct GetCommand<'a> {
    key: &'a str,
}

struct Data {
    flags: u16,
    byte_count: u128,
    data_block: String,
    inserted_time: SystemTime,
    expiry_sec: i128,
}

impl Data {
    fn new(flags: u16, byte_count: u128, data_block: String, expiry_sec: i128) -> Data {
        Data {
            flags,
            byte_count,
            data_block,
            inserted_time: SystemTime::now(),
            expiry_sec,
        }
    }

    fn is_expired(&self) -> bool {
        if self.expiry_sec == 0 {
            return false;
        } else if self.expiry_sec < 0 {
            return true;
        } else {
            let cur_sys_time = SystemTime::now();
            let duration_in_sec = cur_sys_time
                .duration_since(self.inserted_time)
                .unwrap()
                .as_secs() as i128;
            if duration_in_sec > self.expiry_sec {
                return true;
            }
            return false;
        }
    }
}

struct DataStorage {
    store: HashMap<String, Data>,
}

impl DataStorage {
    fn new() -> DataStorage {
        DataStorage {
            store: HashMap::new(),
        }
    }

    fn get(&mut self, key: &str) -> Option<&Data> {
        let should_remove = {
            if let Some(data) = self.store.get(key) {
                data.is_expired()
            } else {
                false
            }
        };

        if should_remove {
            self.remove(key);
            None
        } else {
            self.store.get(key)
        }
    }

    fn exists(&mut self, key: &str) -> bool {
        match self.get(key) {
            Some(_) => true,
            None => false,
        }
    }

    fn insert(&mut self, key: &str, data: Data) -> Option<Data> {
        self.store.insert(key.to_string(), data)
    }

    fn remove(&mut self, key: &str) -> Option<Data> {
        self.store.remove(key)
    }
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

fn handle_connection(sock_stream: TcpStream, data_storage: Arc<Mutex<DataStorage>>) {
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
                                        data.data_block, data.flags, data.byte_count
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
                            set_command.key,
                            Data::new(
                                set_command.flags,
                                set_command.byte_count,
                                set_command.data_block.to_string(),
                                set_command.expiry_date,
                            ),
                        );
                        if set_command.no_reply == false {
                            buf_writer.write_all("STORED\r\n".as_bytes()).unwrap();
                            buf_writer.flush().unwrap();
                        }
                    }
                    Command::Add(mut add_command) => {
                        let mut data = String::new();
                        buf_reader.read_line(&mut data).unwrap();
                        add_command.data_block = data.as_str().trim();

                        let value_exists = data_storage.lock().unwrap().exists(add_command.key);

                        match value_exists {
                            true => {
                                buf_writer.write_all("NOT_STORED\r\n".as_bytes()).unwrap();
                                buf_writer.flush().unwrap();
                            }
                            false => {
                                data_storage.lock().unwrap().insert(
                                    add_command.key,
                                    Data::new(
                                        add_command.flags,
                                        add_command.byte_count,
                                        add_command.data_block.to_string(),
                                        add_command.expiry_date,
                                    ),
                                );

                                buf_writer.write_all("STORED\r\n".as_bytes()).unwrap();
                                buf_writer.flush().unwrap();
                            }
                        }
                    }
                    Command::Replace(mut replace_command) => {
                        let mut data = String::new();
                        buf_reader.read_line(&mut data).unwrap();
                        replace_command.data_block = data.as_str().trim();
                        let value_exists = data_storage.lock().unwrap().exists(replace_command.key);

                        match value_exists {
                            true => {
                                data_storage.lock().unwrap().insert(
                                    replace_command.key,
                                    Data::new(
                                        replace_command.flags,
                                        replace_command.byte_count,
                                        replace_command.data_block.to_string(),
                                        replace_command.expiry_date,
                                    ),
                                );

                                buf_writer.write_all("STORED\r\n".as_bytes()).unwrap();
                                buf_writer.flush().unwrap();
                            }
                            false => {
                                buf_writer.write_all("NOT_STORED\r\n".as_bytes()).unwrap();
                                buf_writer.flush().unwrap();
                            }
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }
}

fn parse_set_add_command(command: Vec<&str>) -> SetAddCommand {
    SetAddCommand {
        key: command.get(1).unwrap(),
        flags: command.get(2).unwrap().parse::<u16>().unwrap(),
        expiry_date: command.get(3).unwrap().parse::<i128>().unwrap(),
        byte_count: command.get(4).unwrap().parse::<u128>().unwrap(),
        no_reply: match command.get(5) {
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
    }
}

fn parse_command(command_string: Vec<&str>) -> Option<Command> {
    match command_string.get(0) {
        Some(str) => {
            if str.eq_ignore_ascii_case("set") {
                let sc = parse_set_add_command(command_string);
                return Some(Command::Set(sc));
            } else if str.eq_ignore_ascii_case("get") {
                let gc = GetCommand {
                    key: command_string.get(1).unwrap(),
                };
                return Some(Command::Get(gc));
            } else if str.eq_ignore_ascii_case("add") {
                let ac = parse_set_add_command(command_string);
                return Some(Command::Add(ac));
            } else if str.eq_ignore_ascii_case("replace") {
                let rc = parse_set_add_command(command_string);
                return Some(Command::Replace(rc));
            }
            return None;
        }
        None => None,
    }
}
fn main() {
    let data_storage = Arc::new(Mutex::new(DataStorage::new()));
    let server = start_server();
    let thread_pool = threads::ThreadPool::new(THREAD_COUNT);

    for sock_stream in server.incoming() {
        let shared_hashmap = Arc::clone(&data_storage);
        thread_pool.execute(move || {
            handle_connection(sock_stream.unwrap(), shared_hashmap);
        });
    }
}
