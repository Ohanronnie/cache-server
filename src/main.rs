use std::{
    collections::HashMap,
    env,
    net::TcpListener,
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
    let host = env::var("CACHE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = env::var("CACHE_PORT").unwrap_or_else(|_| "2222".to_string());
    let addr = format!("{host}:{port}");

    let store: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
    let auth_store: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));

    let username = env::var("CACHE_USERNAME").unwrap_or_else(|_| "admin".to_string());
    let password = env::var("CACHE_PASSWORD").unwrap_or_else(|_| "root".to_string());
    auth_store.lock().unwrap().insert(username, password);

    let listener = TcpListener::bind(&addr)?;
    println!("listening on {addr}");
    for stream in listener.incoming() {
        let mut stream = stream?;
        let store = Arc::clone(&store);
        let auth_store = Arc::clone(&auth_store);

        thread::spawn(move || {
            if let Err(error) = handle_server_connection(&mut stream, &store, &auth_store) {
                eprintln!("connection error: {error}");
            }
        });
    }
    Ok(())
}
// 3$SET$4$NAME$6$RONNIE$
