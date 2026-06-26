use serde_json::Value;

const MSDP_VAR: u8 = 1;
const MSDP_VAL: u8 = 2;
const MSDP_TABLE_OPEN: u8 = 3;
const MSDP_TABLE_CLOSE: u8 = 4;
const MSDP_ARRAY_OPEN: u8 = 5;
const MSDP_ARRAY_CLOSE: u8 = 6;

pub fn parse_msdp(data: &[u8]) -> Value {
    let (val, _) = parse_value(data, 0);
    val
}

fn parse_value(data: &[u8], start: usize) -> (Value, usize) {
    if start >= data.len() {
        return (Value::Null, start);
    }

    let mut vars = serde_json::Map::new();
    let mut i = start;

    while i < data.len() {
        if data[i] == MSDP_VAR {
            i += 1;
            let (name, next) = read_string(data, i);
            i = next;
            if i < data.len() && data[i] == MSDP_VAL {
                i += 1;
                if i < data.len() && data[i] == MSDP_TABLE_OPEN {
                    let (table, next) = parse_table(data, i + 1);
                    vars.insert(name, table);
                    i = next;
                } else if i < data.len() && data[i] == MSDP_ARRAY_OPEN {
                    let (arr, next) = parse_array(data, i + 1);
                    vars.insert(name, arr);
                    i = next;
                } else {
                    let (val, next) = read_string(data, i);
                    vars.insert(name, Value::String(val));
                    i = next;
                }
            }
        } else {
            i += 1;
        }
    }

    (Value::Object(vars), i)
}

fn parse_table(data: &[u8], start: usize) -> (Value, usize) {
    let mut map = serde_json::Map::new();
    let mut i = start;

    while i < data.len() && data[i] != MSDP_TABLE_CLOSE {
        if data[i] == MSDP_VAR {
            i += 1;
            let (name, next) = read_string(data, i);
            i = next;
            if i < data.len() && data[i] == MSDP_VAL {
                i += 1;
                if i < data.len() && data[i] == MSDP_TABLE_OPEN {
                    let (table, next) = parse_table(data, i + 1);
                    map.insert(name, table);
                    i = next;
                } else if i < data.len() && data[i] == MSDP_ARRAY_OPEN {
                    let (arr, next) = parse_array(data, i + 1);
                    map.insert(name, arr);
                    i = next;
                } else {
                    let (val, next) = read_string(data, i);
                    map.insert(name, Value::String(val));
                    i = next;
                }
            }
        } else {
            i += 1;
        }
    }

    if i < data.len() {
        i += 1;
    }
    (Value::Object(map), i)
}

fn parse_array(data: &[u8], start: usize) -> (Value, usize) {
    let mut items = Vec::new();
    let mut i = start;

    while i < data.len() && data[i] != MSDP_ARRAY_CLOSE {
        if data[i] == MSDP_VAL {
            i += 1;
            if i < data.len() && data[i] == MSDP_TABLE_OPEN {
                let (table, next) = parse_table(data, i + 1);
                items.push(table);
                i = next;
            } else if i < data.len() && data[i] == MSDP_ARRAY_OPEN {
                let (arr, next) = parse_array(data, i + 1);
                items.push(arr);
                i = next;
            } else {
                let (val, next) = read_string(data, i);
                items.push(Value::String(val));
                i = next;
            }
        } else {
            i += 1;
        }
    }

    if i < data.len() {
        i += 1;
    }
    (Value::Array(items), i)
}

fn read_string(data: &[u8], start: usize) -> (String, usize) {
    let mut s = String::new();
    let mut i = start;
    while i < data.len() {
        match data[i] {
            MSDP_VAR | MSDP_VAL | MSDP_TABLE_OPEN | MSDP_TABLE_CLOSE | MSDP_ARRAY_OPEN
            | MSDP_ARRAY_CLOSE => break,
            b => {
                s.push(b as char);
                i += 1;
            }
        }
    }
    (s, i)
}

pub fn encode_msdp_var(name: &str, value: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSDP_VAR);
    buf.extend_from_slice(name.as_bytes());
    buf.push(MSDP_VAL);
    buf.extend_from_slice(value.as_bytes());
    buf
}

pub fn encode_msdp_report(vars: &[String]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSDP_VAR);
    buf.extend_from_slice(b"REPORT");
    for var in vars {
        buf.push(MSDP_VAL);
        buf.extend_from_slice(var.as_bytes());
    }
    buf
}

pub fn encode_msdp_send(vars: &[String]) -> Vec<u8> {
    let mut buf = Vec::new();
    buf.push(MSDP_VAR);
    buf.extend_from_slice(b"SEND");
    for var in vars {
        buf.push(MSDP_VAL);
        buf.extend_from_slice(var.as_bytes());
    }
    buf
}
