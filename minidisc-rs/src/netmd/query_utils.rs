use crate::netmd::utils;
use once_cell::sync::Lazy;
use std::collections::hash_map::HashMap;
use std::error::Error;

// prettier-ignore
const FORMAT_TYPE_LEN_DICT: Lazy<HashMap<char, i32>> = Lazy::new(|| {
    HashMap::from([
        ('b', 1), // byte
        ('w', 2), // word
        ('d', 4), // doubleword
        ('q', 8), // quadword
    ])
});

/*
    %b, w, d, q - explained above (can have endiannes overriden by '>' and '<' operators, f. ex. %>d %<q)
    %s - Uint8Array preceded by 2 bytes of length
    %x - Uint8Array preceded by 2 bytes of length
    %z - Uint8Array preceded by 1 byte of length
    %* - raw Uint8Array
    %B - BCD-encoded 1-byte number
    %W - BCD-encoded 2-byte number
*/

const DEBUG: bool = false;

/// Formats a query using a standard input to send to the player
pub fn format_query(
    format: String,
    args: Vec<Option<i64>>,
    array_args: Vec<Vec<u8>>,
) -> Result<Vec<u8>, Box<dyn Error>> {
    if DEBUG {
        println!("SENT>>> F: {}", format);
    }

    let mut result: Vec<u8> = Vec::new();
    let mut half: Option<char> = None;
    let mut arg_stack = args.into_iter();
    let mut array_arg_stack = array_args.into_iter();
    let mut endianness_override: Option<char> = None;

    let mut escaped = false;
    for character in format.chars() {
        if escaped {
            if endianness_override.is_none() && ['<', '>'].contains(&character) {
                endianness_override = Some(character);
                continue;
            }
            escaped = false;
            let value = arg_stack.next().unwrap().unwrap();

            match character {
                character if FORMAT_TYPE_LEN_DICT.contains_key(&character) => {
                    match character {
                        'b' => result.push(value as u8),
                        'w' => {
                            let mut value_bytes = (value as i16).to_be_bytes().to_vec();
                            result.append(&mut value_bytes)
                        }
                        'd' => {
                            let mut value_bytes = (value as i32).to_be_bytes().to_vec();
                            result.append(&mut value_bytes)
                        }
                        'q' => {
                            let mut value_bytes = value.to_be_bytes().to_vec();
                            result.append(&mut value_bytes)
                        }
                        _ => (),
                    };
                    endianness_override = None;
                }
                character if character == 'x' || character == 's' || character == 'z' => {
                    let mut array_value = array_arg_stack.next().unwrap();

                    let mut array_length = array_value.len();

                    if character == 's' {
                        array_length += 1;
                    }

                    if character != 'z' {
                        result.push(((array_length >> 8) & 0xFF) as u8)
                    }
                    result.push((array_length & 0xFF) as u8);
                    result.append(&mut array_value);
                    if character == 's' {
                        result.push(0);
                    }
                }
                character if character == '*' => {
                    let mut array_value = array_arg_stack.next().unwrap();
                    result.append(&mut array_value);
                }
                character if character == 'B' || character == 'W' => {
                    let converted = utils::int_to_bcd(value as i32);
                    if character == 'W' {
                        result.push(((converted >> 8) & 0xFF) as u8);
                    }
                    result.push((converted & 0xFF) as u8);
                }
                _ => return Err(format!("Unrecognized format char {}", character).into()),
            }
            continue;
        }
        if character == '%' {
            escaped = true;
            continue;
        }
        if character == ' ' {
            continue;
        }
        if half.is_none() {
            half = Some(character);
        } else {
            result.push(
                u8::from_str_radix(&String::from_iter([half.unwrap(), character]), 16).unwrap(),
            );
            half = None;
        }
    }

    Ok(result)
}

pub enum QueryResults {
    Number(i64),
    Array(Vec<u8>),
}

impl QueryResults {
    pub fn to_vec(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        match self {
            QueryResults::Array(a) => Ok(a.to_vec()),
            _ => Err("Result was a number".into()),
        }
    }

    pub fn to_i64(&self) -> Result<i64, Box<dyn Error>> {
        match self {
            QueryResults::Number(a) => Ok(*a),
            _ => Err("Result was a number".into()),
        }
    }
}

/// Scans a result using a standard input to recieve from the player
pub fn scan_query(
    query_result: Vec<u8>,
    format: String,
) -> Result<Vec<QueryResults>, Box<dyn Error>> {
    let mut result: Vec<QueryResults> = Vec::new();

    let initial_length = query_result.len();
    let mut input_stack = query_result.into_iter();
    let mut half: Option<char> = None;
    let mut endianness_override: Option<char> = None;
    let mut escaped = false;

    // Remove an unknown byte at the beginning
    // TODO: Find out what this is
    input_stack.next();

    let mut bloop = 0;

    for character in format.chars() {
        if escaped {
            if endianness_override.is_none() && ['<', '>'].contains(&character) {
                endianness_override = Some(character);
                continue;
            }
            escaped = false;

            if character == '?' {
                input_stack.next();
                continue;
            }

            match character {
                character if FORMAT_TYPE_LEN_DICT.contains_key(&character) => {
                    match character {
                        'b' => {
                            let new_value =
                                u8::from_be_bytes(utils::get_bytes(&mut input_stack).unwrap());
                            result.push(QueryResults::Number(new_value as i64));
                        }
                        'w' => {
                            let new_value =
                                i16::from_be_bytes(utils::get_bytes(&mut input_stack).unwrap());
                            result.push(QueryResults::Number(new_value as i64));
                        }
                        'd' => {
                            let new_value =
                                i32::from_be_bytes(utils::get_bytes(&mut input_stack).unwrap());
                            result.push(QueryResults::Number(new_value as i64));
                        }
                        'q' => {
                            let new_value =
                                i64::from_be_bytes(utils::get_bytes(&mut input_stack).unwrap());
                            result.push(QueryResults::Number(new_value));
                        }
                        _ => unreachable!(),
                    };
                    endianness_override = None;
                }
                character if character == 'x' || character == 's' || character == 'z' => {
                    let length = match character {
                        'z' => input_stack.next().unwrap() as u16,
                        _ => u16::from_be_bytes(utils::get_bytes(&mut input_stack).unwrap()),
                    };
                    let mut result_buffer: Vec<u8> = Vec::new();
                    for _ in 0..length {
                        result_buffer.push(input_stack.next().unwrap());
                    }
                    result.push(QueryResults::Array(result_buffer))
                }
                character if character == '*' || character == '#' => {
                    let mut result_buffer: Vec<u8> = Vec::new();
                    let temp_stack = input_stack.clone();
                    for entry in temp_stack.take(initial_length as usize) {
                        result_buffer.push(entry);
                        input_stack.next();
                    }
                    result.push(QueryResults::Array(result_buffer));
                }
                character if character == 'B' => {
                    let v = input_stack.next().unwrap();
                    result.push(QueryResults::Number(utils::bcd_to_int(v as i32) as i64));
                }
                character if character == 'W' => {
                    let v = (input_stack.next().unwrap() as i32) << 8
                        | input_stack.next().unwrap() as i32;
                    result.push(QueryResults::Number(utils::bcd_to_int(v) as i64));
                }
                _ => return Err(format!("Unrecognized format char {}", character).into()),
            }
            continue;
        }
        if character == '%' {
            assert_eq!(half, None);
            escaped = true;
            continue;
        }
        if character == ' ' {
            continue;
        }
        if half.is_none() {
            half = Some(character);
        } else {
            let input_value = input_stack.next().unwrap();
            let format_value =
                u8::from_str_radix(&String::from_iter([half.unwrap(), character]), 16).unwrap();
            if format_value != input_value {
                let i = initial_length - input_stack.len() - 1;
                return Err(format!("Format and input mismatch at {i}: expected {format_value:#0x}, got {input_value:#0x} (format {format})").into());
            }
            half = None;
        }
    }

    assert_eq!(input_stack.len(), 0);
    Ok(result)
}