use crate::utils::*;
use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{SocketAddr, TcpStream},
    sync::{Arc, Mutex},
};

#[derive(Debug)]
struct Message<'a> {
    command: &'a Command,
    key: &'a Key,
    value: Option<Value>,
}
#[derive(Debug, Clone)]
struct Command {
    _length: usize,
    _type: String,
}
#[derive(Debug, Clone)]
struct Key {
    _length: usize,
    _data: String,
}

#[derive(Debug, Clone)]
struct Value {
    _length: usize,
    _data: String,
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
const MAX_DATA_LENGTH: usize = 512;

pub fn handle_server_connection(
    stream: &mut TcpStream,
    store: &Arc<Mutex<HashMap<String, String>>>,
) -> std::io::Result<()> {
    let ip = &stream.peer_addr().unwrap();
    dbg!(ip);
    let mut buffer: Vec<u8> = Vec::new();
    let mut temp = [0u8; MAX_DATA_LENGTH];
    let mut step = Step::CommandLength;
    let mut command: Option<Command> = None;
    let mut key: Option<Key> = None;
    let mut value: Option<Value>;
    let mut message: Option<Message>;
    let (username, password) = ("admin", "root");
    loop {
        let n = stream.read(&mut temp)?;
        if n == 0 {
            println!("Client disconnected");
            break;
        }
        println!("{n}");
        buffer.extend_from_slice(&temp[..n]);

        loop {
            match step {
                Step::CommandLength => {
                    match parse_length(&mut buffer) {
                        Ok(val) => {
                            if let Some(len) = val {
                                step = Step::Command { len };
                            //	break;
                            } else {
                                // if none yet, continue the loop
                                break;
                            }
                        }
                        Err(err) => {
                            stream.write_all(err.as_bytes())?;
                            return Ok(());
                        }
                    };
                }
                Step::Command { len } => match parse_field(&mut buffer, len) {
                    Ok(val) => {
                        if let Some(val) = val {
                            step = Step::KeyLength;
                            command = Option::Some(Command {
                                _length: len,
                                _type: val,
                            });
                        } else {
                            break;
                        }
                    }
                    Err(err) => {
                        stream.write_all(err.as_bytes())?;
                        return Ok(());
                    }
                },
                Step::KeyLength => match parse_length(&mut buffer) {
                    Ok(val) => {
                        if let Some(val) = val {
                            step = Step::Key { len: val }
                        } else {
                            break;
                        }
                    }
                    Err(err) => {
                        stream.write_all(err.as_bytes())?;
                        return Ok(());
                    }
                },
                Step::Key { len } => {
                    match parse_field(&mut buffer, len) {
                        Ok(val) => {
                            if let Some(val) = val {
                                let _key = Key {
                                    _length: len,
                                    _data: val,
                                };
                                let _command = &command.as_ref().unwrap();
                                // if command is not GET or SET
                                // we should reject it
                                if _command._type != "GET"
                                    && _command._type != "SET"
                                    && _command._type != "DELETE"
                                {
                                    stream.write_all(
                                        format!("Invalid action {}", _command._type).as_bytes(),
                                    )?;
                                };
                                key = Some(_key);
                                let _command = command.as_ref().unwrap();
                                let _key = key.as_ref().unwrap();
                                // no need to unwrpa the key, i can just pass the key we defined earlier there.
                                let message = Message {
                                    command: _command,
                                    key: _key,
                                    value: None,
                                };
                                // if command is GET we dont need to process the value section anymore
                                if _command._type == "GET" {
                                    handle_command(message, &store, &stream)?;
                                    step = Step::CommandLength;
                                } else {
                                    step = Step::ValueLength;
                                }
                            } else {
                                break;
                            }
                        }
                        Err(err) => {
                            stream.write_all(err.as_bytes())?;
                            return Ok(());
                        }
                    }
                }
                Step::ValueLength => match parse_length(&mut buffer) {
                    Ok(val) => {
                        if let Some(val) = val {
                            step = Step::Value { len: val }
                        } else {
                            break;
                        }
                    }
                    Err(err) => {
                        stream.write_all(err.as_bytes())?;
                        return Ok(());
                    }
                },
                Step::Value { len } => {
                    match parse_field(&mut buffer, len) {
                        Ok(val) => {
                            if let Some(val) = val {
                                let _value = Value {
                                    _length: len,
                                    _data: val,
                                };
                                let _command = command.as_ref().unwrap();
                                value = Some(_value);
                                //step = Step::Value { command: command.clone(), key: key.clone(), len: *len };
                                message = Some(Message {
                                    command: _command,
                                    key: key.as_ref().unwrap(),
                                    value: value,
                                });

                                // if its set we need value right
                                handle_command(message.unwrap(), &store, &stream)?;
                                step = Step::CommandLength;
                            } else {
                                break;
                            }
                        }
                        Err(err) => {
                            stream.write_all(err.as_bytes())?;
                            return Ok(());
                        }
                    }
                }
            }
            break;
        }
    }
    Ok(())
}
fn handle_command(
    message: Message,
    store: &Arc<Mutex<HashMap<String, String>>>,
    mut stream: &TcpStream,
) -> std::io::Result<()> {
    println!("{:#?}", message);
    let command_type = &message.command._type;
    let key = &message.key._data;
    let value = message.value;
    match command_type.as_str() {
        "GET" => {
            let store = store.lock().unwrap();
            let value = store.get(key);
            if let Some(value) = value {
                stream.write_all(
                    format!(
                        "${}${}${}${}$",
                        "success".len(),
                        "success",
                        value.len(),
                        value
                    )
                    .as_bytes(),
                )?;
            } else {
                let msg = "Not found.";
                stream.write_all(format!("${}${msg}", msg.len()).as_bytes())?;
            }
        }
        "SET" => {
            let mut store = store.lock().unwrap();
            store.insert(key.to_string(), value.unwrap()._data);
            let msg = "success";
            stream.write_all(format!("${}${msg}", msg.len()).as_bytes())?;
        }
        _ => {}
    }
    Ok(())
}

// fn handle_auth(username: &String, password: &String, addr: SocketAddr, auth_store: Mutex<HashMap<String, String>>) -> Result<String, String>{

// 	let store = auth_store.lock().unwrap();

// }
