use std::{io::{Read, Write}, net::{TcpListener, TcpStream}, thread};


// message protocol is
// COMMAND_LENGTH$COMMAND$PAYLOAD_LENGTH$PAYLOAD

// this is the new version i hope will fix the issue there
fn find_dollar(buf: &[u8]) -> Option<usize> {
	buf.iter().position(|v| *v == b'$')
}
// this is the function to wait until it found $ from the stream
fn parse_length(buffer: &mut Vec<u8>) -> Result<Option<usize>, String> {
	let Some(pos) = find_dollar(&buffer) else {
		return Ok(None)
	};
	let length = &buffer[..pos];

	// check if it contains invalid characters then reject it.
	let length_to_string = String::from_utf8(length.to_vec()).map_err(|err| format!("contains invalid characters"))?;
	
	// check if it contains alphabet and not numbers only 
	 if !length_to_string.bytes().all(|v| v.is_ascii_digit()) {
		return Err(format!("the length must be numbers only. "))
	 };
	 // this try to convert the string to a number if not, raises an error
	 let length = length_to_string.parse::<usize>().map_err(|_| format!("unable to convert to number"))?;

	 // remove everything from the buffer including the last $
	 buffer.drain(..=pos);

	 // finally return the length
	 Ok(Some(length))
}
// this try to parse the stream till it gets to the length 
fn parse_field(buffer: &mut Vec<u8>, length: usize) -> Result<Option<String>, String> {
	// if buffer length is less than the expected length, wait for more
	if buffer.len() < length + 1 /* this one is the delimiter $ */ {
		return Ok(None)
	};

	let mut data = buffer[..=length].to_vec();
	if data[length] != b'$' {
		println!("{:#?}", String::from_utf8(data));
		return Err(format!("invalid command!"))
	};
	// remove the $
	data.drain(length..);
	// remove the field we parsed from the buffer
	buffer.drain(..=length);
	let data = String::from_utf8_lossy(&data).to_string();
	return Ok(Some(data))
}

#[derive(Clone, Debug)]
struct Message {
    command: Command,
    payload: Payload,
}
#[derive(Clone, Debug)]
struct Command {
    _length: usize,
    _type: String,
}
#[derive(Clone, Debug)]
struct Payload {
    _length: usize,
    _data: String,
}
enum Step {
	CommandLength,
	Command { 
		len: usize
	},
	PayloadLength {
		command: Command
	},
	Payload {
		len: usize,
		command: Command
	}
}


// only accept a maximum of 512kb data
const MAX_DATA_LENGTH: usize = 512;
fn handle_connection(mut stream: &TcpStream) -> std::io::Result<()> {
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
			match step {
				Step::CommandLength => {
				  match parse_length(&mut buffer) {
						Ok(val ) => {
							if let Some(len) = val {
								step = Step::Command { len };
							//	break;
							} else {
								// if none yet, continue the loop
								continue;
							}
						},
						Err(err) => {
							stream.write_all(err.as_bytes());
							break;
						}
					};
				}, 
				Step::Command { len  } => {
					match parse_field(&mut buffer, len) {
						Ok(val) => {
							 if let Some(val) = val {
								let command = Command {
									_length: len,
									_type: val
								};
								step = Step::PayloadLength { command: command };
							 } else {
								continue;
							 }
						},
						Err(err) => {
							stream.write_all(err.as_bytes());
							break;
						}
					}
				  },
					Step::PayloadLength { ref  command  } => {
					match parse_length(&mut buffer) {
						Ok(val) => {
							 if let Some(val) = val {
							   step = Step::Payload { len: val, command: command.clone() };

							 } else {
								continue;
							 }
						},
						Err(err) => {
							stream.write_all(err.as_bytes());
							break;
						}
					}
				},
					Step::Payload { len, ref command  } => {
					match parse_field(&mut buffer, len) {
						Ok(val) => {
							 if let Some(val) = val {
							   let payload = Payload {
									_data: val,
									_length: len
								 };

								 println!("{:?}", payload);
							 } else {
								continue;
							 }
						},
						Err(err) => {
							stream.write_all(err.as_bytes());
							break;
						}
					}
				}
			}
		}
	}
	Ok(())
}
fn handle_server(listener: TcpListener) -> std::io::Result<()>{
	for stream in listener.incoming() {
		let stream = stream?;
		handle_connection(&stream)?;
	}
	Ok(())
}
fn handle_client(mut stream: TcpStream) -> std::io::Result<()> {

	Ok(())
}
fn main() -> std::io::Result<()> {
	let addr = "127.0.0.1:2222";
	let listener = TcpListener::bind(addr)?;
	println!("listening on {addr}");
	let server_thread = thread::spawn(move || {
		handle_server(listener)
	});
	let client_thread = thread::spawn(move || {
		let mut stream = TcpStream::connect(addr)?;
		handle_client(stream)
	});

	let _ = server_thread.join().unwrap();
	let _ = client_thread.join().unwrap();

	Ok(())
}