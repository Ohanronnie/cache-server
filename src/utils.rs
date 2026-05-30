pub fn find_dollar(buf: &[u8]) -> Option<usize> {
    buf.iter().position(|v| *v == b'$')
}

// this is the function to wait until it found $ from the stream
pub fn parse_length(buffer: &mut Vec<u8>) -> Result<Option<usize>, String> {
    let Some(pos) = find_dollar(&buffer) else {
        return Ok(None);
    };
    let length = &buffer[..pos];

    // check if it contains invalid characters then reject it.
    let length_to_string =
        String::from_utf8(length.to_vec()).map_err(|_| format!("contains invalid characters"))?;

    // check if it contains alphabet and not numbers only
    if !length_to_string.bytes().all(|v| v.is_ascii_digit()) {
        buffer.drain(..=pos);
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
pub fn parse_field(buffer: &mut Vec<u8>, length: usize) -> Result<Option<String>, String> {
    // if buffer length is less than the expected length, wait for more
    if buffer.len() < length + 1
    /* this one is the delimiter $ */
    {
        return Ok(None);
    };

    let mut data = buffer[..=length].to_vec();
    if data[length] != b'$' {
        println!("{:#?}", String::from_utf8(data));
        buffer.drain(..=length);
        return Err(format!("invalid command!"));
    };
    // remove the $
    data.drain(length..);
    // remove the field we parsed from the buffer
    buffer.drain(..=length);
    let data = String::from_utf8_lossy(&data).to_string();
    return Ok(Some(data));
}
