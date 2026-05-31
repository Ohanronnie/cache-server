use crate::utils::*;
use std::{
    collections::HashMap,
    io::{Error, ErrorKind, Read, Write},
    net::TcpStream,
    sync::{Arc, Mutex},
};

const MAX_DATA_LENGTH: usize = 512;

#[derive(Debug)]
struct Request {
    command: String,
    key: String,
    value: Option<String>,
}

#[derive(Debug)]
enum Step {
    CommandLength,
    Command { len: usize },
    KeyLength,
    Key { len: usize },
    ValueLength,
    Value { len: usize },
}

enum Response {
    Error(String),
    Success(String),
}

pub fn handle_server_connection(
    stream: &mut TcpStream,
    store: &Arc<Mutex<HashMap<String, String>>>,
    auth_store: &Arc<Mutex<HashMap<String, String>>>,
) -> std::io::Result<()> {
    let ip = stream.peer_addr()?;
    println!("client connected: {ip}");

    let mut buffer: Vec<u8> = Vec::new();
    let mut temp = [0u8; MAX_DATA_LENGTH];
    let mut state = ConnectionState::new();

    loop {
        match stream.read(&mut temp) {
            Ok(0) => {
                println!("client disconnected: {ip}");
                break;
            }
            Ok(n) => buffer.extend_from_slice(&temp[..n]),
            Err(err) if err.kind() == ErrorKind::ConnectionReset => {
                println!("client reset connection: {ip}");
                break;
            }
            Err(err) => return Err(err),
        }

        if let Err(error) = handle_request(stream, &mut state, &mut buffer, store, auth_store) {
            stream.write_all(format_response(Response::Error(error.to_string())).as_bytes())?;
            return Ok(());
        }
    }

    Ok(())
}

struct ConnectionState {
    step: Step,
    command: Option<String>,
    key: Option<String>,
    authenticated: bool,
}

impl ConnectionState {
    fn new() -> Self {
        Self {
            step: Step::CommandLength,
            command: None,
            key: None,
            authenticated: false,
        }
    }

    fn reset_request(&mut self) {
        self.step = Step::CommandLength;
        self.command = None;
        self.key = None;
    }
}

fn handle_request(
    stream: &mut TcpStream,
    state: &mut ConnectionState,
    buffer: &mut Vec<u8>,
    store: &Arc<Mutex<HashMap<String, String>>>,
    auth_store: &Arc<Mutex<HashMap<String, String>>>,
) -> std::io::Result<()> {
    loop {
        match state.step {
            Step::CommandLength => {
                let Some(len) = parse_length(buffer).map_err(invalid_data)? else {
                    break;
                };
                state.step = Step::Command { len };
            }
            Step::Command { len } => {
                let Some(command) = parse_field(buffer, len).map_err(invalid_data)? else {
                    break;
                };
                state.command = Some(command.to_ascii_uppercase());
                state.step = Step::KeyLength;
            }
            Step::KeyLength => {
                let Some(len) = parse_length(buffer).map_err(invalid_data)? else {
                    break;
                };
                state.step = Step::Key { len };
            }
            Step::Key { len } => {
                let Some(key) = parse_field(buffer, len).map_err(invalid_data)? else {
                    break;
                };
                state.key = Some(key);

                if command_has_value(state.command.as_deref()) {
                    state.step = Step::ValueLength;
                } else {
                    let response = handle_command(
                        state,
                        Request {
                            command: state.command.clone().unwrap_or_default(),
                            key: state.key.clone().unwrap_or_default(),
                            value: None,
                        },
                        store,
                        auth_store,
                    );
                    stream.write_all(response.as_bytes())?;
                    state.reset_request();
                }
            }
            Step::ValueLength => {
                let Some(len) = parse_length(buffer).map_err(invalid_data)? else {
                    break;
                };
                state.step = Step::Value { len };
            }
            Step::Value { len } => {
                let Some(value) = parse_field(buffer, len).map_err(invalid_data)? else {
                    break;
                };
                let response = handle_command(
                    state,
                    Request {
                        command: state.command.clone().unwrap_or_default(),
                        key: state.key.clone().unwrap_or_default(),
                        value: Some(value),
                    },
                    store,
                    auth_store,
                );
                stream.write_all(response.as_bytes())?;
                state.reset_request();
            }
        }
    }

    Ok(())
}

fn invalid_data(message: String) -> Error {
    Error::new(ErrorKind::InvalidData, message)
}

fn command_has_value(command: Option<&str>) -> bool {
    matches!(command, Some("AUTH" | "SET"))
}

fn format_response(response: Response) -> String {
    let (response_type, value) = match response {
        Response::Error(value) => ("ERROR", value),
        Response::Success(value) => ("RESPONSE", value),
    };

    format!(
        "{}${}${}${}$",
        response_type.len(),
        response_type,
        value.len(),
        value
    )
}

fn handle_command(
    state: &mut ConnectionState,
    request: Request,
    store: &Arc<Mutex<HashMap<String, String>>>,
    auth_store: &Arc<Mutex<HashMap<String, String>>>,
) -> String {
    match request.command.as_str() {
        "AUTH" => handle_auth(state, &request, auth_store),
        "GET" | "SET" | "DELETE" if !state.authenticated => {
            format_response(Response::Error("authentication required".to_string()))
        }
        "GET" => {
            let store = store.lock().unwrap();
            let value = store.get(&request.key).cloned().unwrap_or_default();
            format_response(Response::Success(value))
        }
        "SET" => {
            let mut store = store.lock().unwrap();
            let value = request.value.unwrap_or_default();
            store.insert(request.key, value);
            format_response(Response::Success(String::new()))
        }
        "DELETE" => {
            let mut store = store.lock().unwrap();
            store.remove(&request.key);
            format_response(Response::Success(String::new()))
        }
        _ => format_response(Response::Error(format!(
            "invalid command {}",
            request.command
        ))),
    }
}

fn handle_auth(
    state: &mut ConnectionState,
    request: &Request,
    auth_store: &Arc<Mutex<HashMap<String, String>>>,
) -> String {
    let Some(password) = request.value.as_ref() else {
        return format_response(Response::Error("password is required".to_string()));
    };

    let auth_store = auth_store.lock().unwrap();
    match auth_store.get(&request.key) {
        Some(stored_password) if stored_password == password => {
            state.authenticated = true;
            format_response(Response::Success("OK".to_string()))
        }
        _ => format_response(Response::Error("invalid credentials".to_string())),
    }
}
