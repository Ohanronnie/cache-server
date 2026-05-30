use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    vec,
};

// this is my first trial on building it but it has some issues
// the length "length" must be exactly 1,
// else it wont work, 
// so we cant pass a large data to it,
// the max payload length would be capped at 9

#[derive(Clone, Debug)]
struct Message {
    command: Command,
    payload: Payload,
}
#[derive(Clone, Debug)]
struct Command {
    _length: u8,
    _type: String,
}
#[derive(Clone, Debug)]
struct Payload {
    _length: u8,
    _data: String,
}
enum Step {
    CommandLength,
    Command,
    PayloadLength,
    Payload,
}

// message protocol is
// COMMAND_LENGTH$COMMAND$PAYLOAD_LENGTH$PAYLOAD

fn handle_stream(mut stream: TcpStream) {
    let error_message = "Error parsing command!\n".as_bytes();
    let mut buffer: Vec<u8> = Vec::new();

    // this for storing the length of the expected buffer
    let mut next_length = 1;
    let mut prev_length = 0;
    let mut step = Step::CommandLength;

    let mut command: Option<Command> = Option::None;
    let mut payload: Option<Payload> = Option::None;
    let mut message: Option<Message> = Option::None;

    loop {
        let mut temp = vec![0u8; next_length + 1 /* the 1 is to account for the extra $ that will be at the end of every command*/];
        let n = stream.read(&mut temp).unwrap();

        if n == 0 {
            println!("client disconnected");
            break;
        }

        let mut data = String::from_utf8_lossy(&temp[..n]).into_owned();
        // get the last characteer and see if its $ or not.
        if let Option::Some(v) = data.chars().nth(next_length) {
            if v != '$' {
                println!("{v}");
                stream.write_all(error_message).unwrap();
                break;
            } else {
                // remove the $
                data.pop();
            }
        } else {
            println!("error here, {next_length} ");
            stream.write_all(error_message).unwrap();
        }
        match step {
            Step::CommandLength => {
                // the command length
                let length = data.chars().nth(0).expect("failed to get the 1st value");
                let length = length.to_digit(10);
                match length {
                    Option::Some(length) => {
                        prev_length = next_length as u8;
                        next_length = length as usize;
                        step = Step::Command;
                    }
                    Option::None => {
                        stream.write_all(error_message);
                        break;
                    }
                }
            }
            Step::Command => {
                command = Option::Some(Command {
                    _length: next_length as u8,
                    _type: data,
                });
                prev_length = next_length as u8;
                next_length = 1;
                step = Step::PayloadLength;
            }
            Step::PayloadLength => {
                let length = data.chars().nth(0).expect("failed to get the 1st value");
                let length = length.to_digit(10);
                match length {
                    Option::Some(length) => {
                        prev_length = next_length as u8;
                        next_length = length as usize;
                        step = Step::Payload;
                    }
                    Option::None => {
                        stream.write_all(error_message);
                        break;
                    }
                }
            }
            Step::Payload => {
                payload = Option::Some(Payload {
                    _length: next_length as u8,
                    _data: data,
                });
                message = Option::Some(Message {
                    command: command.clone().expect("command not found"),
                    payload: payload.clone().expect("payload not found"),
                });

                println!("{:?}", message);
                step = Step::CommandLength;
            }
            _ => {}
        }
    }
}
fn main() {
    let listener = TcpListener::bind("127.0.0.1:2222").unwrap();
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_stream(stream);
    }
}
