use serde_json::Value;

pub struct GmcpMessage {
  pub package: String,
  pub data: Value
}

pub fn parse_gmcp(data: &[u8]) -> Option<GmcpMessage> {
  let text = String::from_utf8_lossy(data);
  let space_pos = text.find(' ').unwrap_or(text.len());
  let package = text[..space_pos].to_string();
  let data = if space_pos < text.len() {
    serde_json::from_str(&text[space_pos + 1..]).unwrap_or(Value::Null)
  } else {
    Value::Null
  };
  Some(GmcpMessage { package, data })
}

pub fn encode_gmcp(package: &str, data: &Value) -> Vec<u8> {
  let json = serde_json::to_string(data).unwrap_or_default();
  if json == "null" {
    package.as_bytes().to_vec()
  } else {
    format!("{package} {json}").into_bytes()
  }
}
