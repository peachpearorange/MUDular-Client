use std::collections::HashMap;

const MSSP_VAR: u8 = 1;
const MSSP_VAL: u8 = 2;

pub fn parse_mssp(data: &[u8]) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut i = 0;
    let mut current_var = String::new();

    while i < data.len() {
        match data[i] {
            MSSP_VAR => {
                i += 1;
                current_var.clear();
                while i < data.len() && data[i] != MSSP_VAL && data[i] != MSSP_VAR {
                    current_var.push(data[i] as char);
                    i += 1;
                }
            }
            MSSP_VAL => {
                i += 1;
                let mut val = String::new();
                while i < data.len() && data[i] != MSSP_VAR && data[i] != MSSP_VAL {
                    val.push(data[i] as char);
                    i += 1;
                }
                if !current_var.is_empty() {
                    result.insert(current_var.clone(), val);
                }
            }
            _ => i += 1,
        }
    }

    result
}
