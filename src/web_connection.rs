use {std::cell::RefCell,
     std::rc::Rc,
     std::sync::mpsc,
     wasm_bindgen::{JsCast, closure::Closure},
     web_sys::WebSocket};

use crate::{protocol::{gmcp, msdp, mssp},
            telnet::*};

pub enum ConnEvent {
  Connected,
  Data(String),
  PendingData(String),
  GmcpReceived(String, serde_json::Value),
  MsspReceived(std::collections::HashMap<String, String>),
  MsdpReceived(serde_json::Value),
  Disconnected(String),
  Error(String)
}

struct TelnetState {
  parser: TelnetParser,
  partial_line: String,
  gmcp_enabled: bool,
  binary_mode: bool,
}

pub struct Connection {
  pub event_rx: mpsc::Receiver<ConnEvent>,
  ws: WebSocket,
  state: Rc<RefCell<TelnetState>>,
  _onopen: Closure<dyn FnMut(web_sys::Event)>,
  _onmessage: Closure<dyn FnMut(web_sys::MessageEvent)>,
  _onerror: Closure<dyn FnMut(web_sys::ErrorEvent)>,
  _onclose: Closure<dyn FnMut(web_sys::Event)>
}

impl Connection {
  pub fn connect(url: &str, protocol: Option<&str>) -> Result<Self, String> {
    let ws = protocol
      .map(|protocol| WebSocket::new_with_str(url, protocol))
      .unwrap_or_else(|| WebSocket::new(url))
      .map_err(|_| format!("Could not open WebSocket {url}"))?;
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let (event_tx, event_rx) = mpsc::channel();
    let state = Rc::new(RefCell::new(TelnetState {
      parser: TelnetParser::new(),
      partial_line: String::new(),
      gmcp_enabled: false,
      binary_mode: false,
    }));

    let tx = event_tx.clone();
    let onopen = Closure::wrap(Box::new(move |_: web_sys::Event| {
      let _ = tx.send(ConnEvent::Connected);
    }) as Box<dyn FnMut(_)>);
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));

    let tx = event_tx.clone();
    let ws_ref = ws.clone();
    let st = state.clone();
    let onmessage = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
      let data = event.data();
      if data.is_instance_of::<js_sys::ArrayBuffer>() {
        let bytes = js_sys::Uint8Array::new(&data).to_vec();
        let mut st = st.borrow_mut();
        st.binary_mode = true;
        process_binary(&bytes, &mut st, &tx, &ws_ref);
      } else if let Some(text) = data.as_string() {
        for line in websocket_text_lines(&text) {
          let _ = tx.send(ConnEvent::Data(line));
        }
      }
    }) as Box<dyn FnMut(_)>);
    ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));

    let tx = event_tx.clone();
    let onerror = Closure::wrap(Box::new(move |event: web_sys::ErrorEvent| {
      let message = event.message();
      let detail = if message.is_empty() { "WebSocket error".into() } else { message };
      let _ = tx.send(ConnEvent::Error(detail));
    }) as Box<dyn FnMut(_)>);
    ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));

    let tx = event_tx;
    let onclose = Closure::wrap(Box::new(move |_: web_sys::Event| {
      let _ = tx.send(ConnEvent::Disconnected("WebSocket closed".into()));
    }) as Box<dyn FnMut(_)>);
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));

    Ok(Self {
      event_rx,
      ws,
      state,
      _onopen: onopen,
      _onmessage: onmessage,
      _onerror: onerror,
      _onclose: onclose
    })
  }

  pub fn send(&self, text: &str) {
    if self.state.borrow().binary_mode {
      let mut data = text.as_bytes().to_vec();
      data.extend_from_slice(b"\r\n");
      let _ = self.ws.send_with_u8_array(&data);
    } else {
      let _ = self.ws.send_with_str(text);
    }
  }

  pub fn send_gmcp(&self, package: &str, data: &serde_json::Value) {
    let payload = gmcp::encode_gmcp(package, data);
    let _ = self.ws.send_with_u8_array(&TelnetParser::build_subneg(OPT_GMCP, &payload));
  }

  pub fn send_msdp_report(&self, vars: Vec<String>) {
    let payload = msdp::encode_msdp_report(&vars);
    let _ = self.ws.send_with_u8_array(&TelnetParser::build_subneg(OPT_MSDP, &payload));
  }

  pub fn send_msdp_send(&self, vars: Vec<String>) {
    let payload = msdp::encode_msdp_send(&vars);
    let _ = self.ws.send_with_u8_array(&TelnetParser::build_subneg(OPT_MSDP, &payload));
  }

  pub fn send_msdp_list(&self, what: String) {
    let payload = msdp::encode_msdp_var("LIST", &what);
    let _ = self.ws.send_with_u8_array(&TelnetParser::build_subneg(OPT_MSDP, &payload));
  }

  pub fn disconnect(&self) { let _ = self.ws.close(); }

  pub fn poll_events(&self) -> Vec<ConnEvent> {
    let mut events = Vec::new();
    while let Ok(ev) = self.event_rx.try_recv() {
      events.push(ev);
    }
    events
  }
}

fn process_binary(
  bytes: &[u8],
  st: &mut TelnetState,
  tx: &mpsc::Sender<ConnEvent>,
  ws: &WebSocket
) {
  for event in st.parser.parse(bytes) {
    match event {
      TelnetEvent::Data(data) => {
        let text = String::from_utf8_lossy(&data);
        st.partial_line.push_str(&text);
        while let Some(pos) = st.partial_line.find('\n') {
          let line = st.partial_line[..pos].trim_end_matches('\r').to_string();
          st.partial_line = st.partial_line[pos + 1..].to_string();
          let _ = tx.send(ConnEvent::Data(line));
        }
        let _ = tx.send(ConnEvent::PendingData(st.partial_line.clone()));
      }
      TelnetEvent::Negotiate(cmd, opt) => {
        for resp in handle_negotiation(cmd, opt, &mut st.gmcp_enabled) {
          let _ = ws.send_with_u8_array(&resp);
        }
      }
      TelnetEvent::Subnegotiation(opt, data) => match opt {
        OPT_GMCP => {
          if let Some(msg) = gmcp::parse_gmcp(&data) {
            let _ = tx.send(ConnEvent::GmcpReceived(msg.package, msg.data));
          }
        }
        OPT_MSSP => {
          let _ = tx.send(ConnEvent::MsspReceived(mssp::parse_mssp(&data)));
        }
        OPT_MSDP => {
          let _ = tx.send(ConnEvent::MsdpReceived(msdp::parse_msdp(&data)));
        }
        OPT_TTYPE => {
          if data.first() == Some(&1) {
            let mut resp = vec![0u8];
            resp.extend_from_slice(b"MUDular");
            let _ = ws.send_with_u8_array(&TelnetParser::build_subneg(OPT_TTYPE, &resp));
          }
        }
        _ => {}
      },
      TelnetEvent::GoAhead => {
        if !st.partial_line.is_empty() {
          let line = std::mem::take(&mut st.partial_line);
          let _ = tx.send(ConnEvent::Data(line));
          let _ = tx.send(ConnEvent::PendingData(String::new()));
        }
      }
    }
  }
}

fn handle_negotiation(cmd: u8, opt: u8, gmcp_enabled: &mut bool) -> Vec<Vec<u8>> {
  let mut responses = Vec::new();
  match cmd {
    WILL => match opt {
      OPT_MCCP2 => responses.push(TelnetParser::build_dont(opt)),
      OPT_MSSP => responses.push(TelnetParser::build_do(opt)),
      OPT_MSDP => {
        responses.push(TelnetParser::build_do(opt));
        let list_query = msdp::encode_msdp_var("LIST", "REPORTABLE_VARIABLES");
        responses.push(TelnetParser::build_subneg(OPT_MSDP, &list_query));
      }
      OPT_GMCP => {
        *gmcp_enabled = true;
        responses.push(TelnetParser::build_do(opt));
        let hello = gmcp::encode_gmcp(
          "Core.Hello",
          &serde_json::json!({"client": "MUDular", "version": "0.1.0"})
        );
        responses.push(TelnetParser::build_subneg(OPT_GMCP, &hello));
      }
      OPT_SGA | OPT_ECHO => responses.push(TelnetParser::build_do(opt)),
      _ => responses.push(TelnetParser::build_dont(opt))
    },
    DO => match opt {
      OPT_TTYPE | OPT_NAWS | OPT_SGA => responses.push(TelnetParser::build_will(opt)),
      OPT_GMCP => {
        *gmcp_enabled = true;
        responses.push(TelnetParser::build_will(opt));
      }
      _ => responses.push(TelnetParser::build_wont(opt))
    },
    WONT => responses.push(TelnetParser::build_dont(opt)),
    DONT => responses.push(TelnetParser::build_wont(opt)),
    _ => {}
  }
  responses
}

fn websocket_text_lines(text: &str) -> Vec<String> {
  serde_json::from_str::<serde_json::Value>(text)
    .ok()
    .and_then(|message| {
      message.get("events").and_then(|events| events.as_array()).map(|events| {
        events
          .iter()
          .filter_map(|event| {
            (event.get("type").and_then(|value| value.as_str()) == Some("text"))
              .then(|| event.get("text").and_then(|value| value.as_str()))
              .flatten()
          })
          .flat_map(|text| {
            htmlish_to_text(text).lines().map(str::to_string).collect::<Vec<_>>()
          })
          .collect::<Vec<_>>()
      })
    })
    .filter(|lines| !lines.is_empty())
    .unwrap_or_else(|| text.lines().map(str::to_string).collect())
}

fn htmlish_to_text(text: &str) -> String {
  let mut out = String::new();
  let mut chars = text.chars().peekable();
  while let Some(ch) = chars.next() {
    if ch == '<' {
      let mut tag = String::new();
      for tag_ch in chars.by_ref() {
        if tag_ch == '>' {
          break;
        }
        tag.push(tag_ch);
      }
      if tag.trim_start().starts_with("br") {
        out.push('\n');
      }
    } else if ch == '&' {
      let mut entity = String::new();
      while let Some(&entity_ch) = chars.peek() {
        chars.next();
        if entity_ch == ';' {
          break;
        }
        entity.push(entity_ch);
      }
      match entity.as_str() {
        "lt" => out.push('<'),
        "gt" => out.push('>'),
        "amp" => out.push('&'),
        "quot" => out.push('"'),
        _ => {
          out.push('&');
          out.push_str(&entity);
          out.push(';');
        }
      }
    } else {
      out.push(ch);
    }
  }
  out
}
