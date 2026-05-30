use std::{
    collections::HashMap,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread,
};
mod server;
mod utils;
use server::handle_server_connection;

// message protocol is
// COMMAND_LENGTH$COMMAND$PAYLOAD_LENGTH$PAYLOAD

/*
 command format is this

 COMMAND_LENGTH $ COMMAND $ KEY_LENGTH $ KEY $ VALUE_LENGTH $ VALUE $

 3 $ GET $ 4 $ NAME $ 6 $ RONNIE $

 without the whitespace ofcourse

 3$GET$4$NAME$6$RONNIE$

 now command can be either GET (READ) or SET (WRITE)

 so if the command is GET, we ignore the value since its not necessary needed.

 but if its SET, we need the value right? yeah.
*/
fn main() -> std::io::Result<()> {
    let addr = "127.0.0.1:2222";

    let store: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    let auth_store: Mutex<HashMap<String, String>> = Mutex::new(HashMap::new());
    let listener = TcpListener::bind(addr)?;
    println!("listening on {addr}");
    for stream in listener.incoming() {
        let mut stream = stream?;
        handle_server_connection(&mut stream, &store)?;
    }

    Ok(())
}
// 3$SET$4$NAME$6$RONNIE$
