use std::{
    collections::HashMap, io::{Read, Write}, net::{TcpListener, TcpStream}, sync::{Arc, Mutex}, thread
};

// message protocol is
// COMMAND_LENGTH$COMMAND$PAYLOAD_LENGTH$PAYLOAD

// this is the new version i hope will fix the issue there
fn find_dollar(buf: &[u8]) -> Option<usize> {
    buf.iter().position(|v| *v == b'$')
}
// this is the function to wait until it found $ from the stream
fn parse_length(buffer: &mut Vec<u8>) -> Result<Option<usize>, String> {
    let Some(pos) = find_dollar(&buffer) else {
        return Ok(None);
    };
    let length = &buffer[..pos];

    // check if it contains invalid characters then reject it.
    let length_to_string =
        String::from_utf8(length.to_vec()).map_err(|err| format!("contains invalid characters"))?;

    // check if it contains alphabet and not numbers only
    if !length_to_string.bytes().all(|v| v.is_ascii_digit()) {
        return Err(format!("the length must be numbers only. "));
    };
    // this try to convert the string to a number if not, raises an error
    let length = length_to_string
        .parse::<usize>()
        .map_err(|_| format!("unable to convert to number"))?;

    // remove everything from the buffer including the last $
    buffer.drain(..=pos);

    // finally return the length
    Ok(Some(length))
}
// this try to parse the stream till it gets to the length
fn parse_field(buffer: &mut Vec<u8>, length: usize) -> Result<Option<String>, String> {
    // if buffer length is less than the expected length, wait for more
    if buffer.len() < length + 1
    /* this one is the delimiter $ */
    {
        return Ok(None);
    };

    let mut data = buffer[..=length].to_vec();
    if data[length] != b'$' {
        println!("{:#?}", String::from_utf8(data));
        return Err(format!("invalid command!"));
    };
    // remove the $
    data.drain(length..);
    // remove the field we parsed from the buffer
    buffer.drain(..=length);
    let data = String::from_utf8_lossy(&data).to_string();
    return Ok(Some(data));
}

#[derive(Clone, Debug)]
struct Message {
    command: Command,
    key: Key,
    value: Value,
}
#[derive(Clone, Debug)]
struct Command {
    _length: usize,
    _type: String,
}
#[derive(Clone, Debug)]
struct Key {
    _length: usize,
    _data: String,
}

#[derive(Clone, Debug)]
struct Value {
    _length: usize,
    _data: String,
}
#[derive(Debug)]
enum Step {
    CommandLength,
    Command {
        len: usize,
    },

    KeyLength {
        command: Command,
    },
    Key {
        command: Command,
        len: usize,
    },

    ValueLength {
        command: Command,
        key: Key,
    },
    Value {
        command: Command,
        key: Key,
        len: usize,
    },
		 Done {
        command: Command,
        key: Key,
        len: usize,
				value: Value
    },
}
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
// only accept a maximum of 512kb data
const MAX_DATA_LENGTH: usize = 512;
fn handle_connection(mut stream: &TcpStream, store: &Arc<Mutex<HashMap<String, String>>>) -> std::io::Result<()> {
    let mut buffer: Vec<u8> = Vec::new();
    let mut temp = [0u8; MAX_DATA_LENGTH];
    let mut step = Step::CommandLength;

    loop {
        let n = stream.read(&mut temp)?;
        if n == 0 {
            println!("Client disconnected");
            break;
        }
        buffer.extend_from_slice(&temp[..n]);

        loop {
            match &step {
                Step::CommandLength => {
                    match parse_length(&mut buffer) {
                        Ok(val) => {
                            if let Some(len) = val {
                                step = Step::Command { len };
                            //	break;
                            } else {
                                // if none yet, continue the loop
                                continue;
                            }
                        }
                        Err(err) => {
                            stream.write_all(err.as_bytes())?;
                            break;
                        }
                    };
                }
                Step::Command { len } => match parse_field(&mut buffer, *len) {
                    Ok(val) => {
                        if let Some(val) = val {
                            let command = Command {
                                _length: *len,
                                _type: val,
                            };
                            step = Step::KeyLength { command }
                        } else {
                            continue;
                        }
                    }
                    Err(err) => {
                        stream.write_all(err.as_bytes())?;
                        break;
                    }
                },
								Step::KeyLength { command } => {
									match parse_length(&mut buffer) {
										Ok(val ) => {
											if let Some(val) = val {
												step = Step::Key { command: command.clone(), len: val }
											} else {
												continue;
											}
										},
										Err(err) => {
											stream.write_all(err.as_bytes())?;
											break;
										}
									}
								},
								Step::Key { command, len } => {
									match parse_field(&mut buffer, *len) {
											Ok(val) => {
												if let Some(val) = val {
													let key = Key {
														_length: *len,
														_data: val
													};
													// if command is not GET or SET 
													// we should reject it 
													if command._type != "GET" && command._type != "SET" {
														stream.write_all(format!("Invalid action {}", command._type).as_bytes())?;
														break;
													};
													// if command is GET we dont need to process the value section anymore
													if command._type == "GET" {
														 
													  handle_command(Step::ValueLength { command: command.clone(), key }, &store, stream);
														break;
													} else {
														step = Step::ValueLength { command: command.clone(), key };
													}
												

												} else {
													continue;
												}
											},
											Err(err) => {
												stream.write_all(err.as_bytes())?;
												break;
											}
									}
								},
								Step::ValueLength { command, key } => {
									match parse_length(&mut buffer) {
											Ok(val) => {
												if let Some(val) = val {
													step = Step::Value { command: command.clone(), key: key.clone(), len: val }
												} else {
													continue
												}
											},
												Err(err) => {
												stream.write_all(err.as_bytes())?;
												break;
											}
									}
								},
								Step::Value { command, key, len } => {
									match parse_field(&mut buffer, *len) {
										Ok(val) => {
											if let Some(val) = val {
												let value = Value {
													_length: *len,
													_data: val
												};
												//step = Step::Value { command: command.clone(), key: key.clone(), len: *len };

												// if its set we need value right
												handle_command(Step::Done { command: command.clone(), key: key.clone(), len: *len, value: value.clone() }, &store, &stream);
												break;
											} else {
												continue;
											}
										},
										Err(err) => {
											stream.write_all(err.as_bytes())?;
											break;
										}
									}
								},
								_ => {}
            }
        }
    }
    Ok(())
}
fn handle_command(command: Step, store: &Arc<Mutex<HashMap<String, String>>>, mut stream: &TcpStream ) -> std::io::Result<()>{
	println!("{:#?}", command);

	match command {
		// value length is GET 
		// will rewrite soon just a rough sketch
		Step::ValueLength { command, key } => {
			let mut store = store.lock().unwrap();
			let value = store.get(&key._data);
			if let Some(value) = value {
				stream.write_all(format!("${}${}${}${}$","success".len(),"success", value.len(), value).as_bytes())?;
			} else {
				let msg = "Not found.";
				stream.write_all(format!("${}${msg}",msg.len()).as_bytes())?;
			}
		},
		// this is the set
		Step::Done { command, key, len, value } => {
				let mut store = store.lock().unwrap();
			  store.insert(key._data, value._data);
				let msg = "success";
				stream.write_all(format!("${}${msg}",msg.len()).as_bytes())?;
		}
		_ => {}
	}
	Ok(())
}
fn handle_server(listener: &TcpListener, store: &Arc<Mutex<HashMap<String, String>>>) -> std::io::Result<()> {
    for stream in listener.incoming() {
        let stream = stream?;
        handle_connection(&stream, &store)?;
    }
    Ok(())
}
fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {
    Ok(())
}
fn main() -> std::io::Result<()> {
    let addr = "127.0.0.1:2222";
    let listener = TcpListener::bind(addr)?;
		let mut store: Arc<Mutex<HashMap<String, String>>> = Arc::new(Mutex::new(HashMap::new()));
		
    println!("listening on {addr}");
    let server_thread = thread::spawn(move || handle_server(&listener, &store));
    let client_thread = thread::spawn(move || {
        let mut stream = TcpStream::connect(addr)?;
        handle_client(stream)
    });

    let _ = server_thread.join().unwrap();
    let _ = client_thread.join().unwrap();

    Ok(())
}
// 3$SET$4$NAME$6$RONNIE$