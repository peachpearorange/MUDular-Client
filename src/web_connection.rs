use {std::sync::mpsc,
     wasm_bindgen::{JsCast, closure::Closure},
     web_sys::WebSocket};

pub enum ConnEvent {
  Connected,
  Data(String),
  Disconnected(String),
  Error(String)
}

pub struct Connection {
  pub event_rx: mpsc::Receiver<ConnEvent>,
  ws: WebSocket,
  _onopen: Closure<dyn FnMut(web_sys::Event)>,
  _onmessage: Closure<dyn FnMut(web_sys::MessageEvent)>,
  _onerror: Closure<dyn FnMut(web_sys::ErrorEvent)>,
  _onclose: Closure<dyn FnMut(web_sys::Event)>
}

impl Connection {
  pub fn connect(url: &str) -> Result<Self, String> {
    let ws =
      WebSocket::new(url).map_err(|_| format!("Could not open WebSocket {url}"))?;
    ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

    let (event_tx, event_rx) = mpsc::channel();

    let tx = event_tx.clone();
    let onopen = Closure::wrap(Box::new(move |_event: web_sys::Event| {
      let _ = tx.send(ConnEvent::Connected);
    }) as Box<dyn FnMut(_)>);
    ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));

    let tx = event_tx.clone();
    let onmessage = Closure::wrap(Box::new(move |event: web_sys::MessageEvent| {
      if let Some(text) = event.data().as_string() {
        for line in websocket_text_lines(&text) {
          let _ = tx.send(ConnEvent::Data(line.to_string()));
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
    let onclose = Closure::wrap(Box::new(move |_event: web_sys::Event| {
      let _ = tx.send(ConnEvent::Disconnected("WebSocket closed".into()));
    }) as Box<dyn FnMut(_)>);
    ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));

    Ok(Self {
      event_rx,
      ws,
      _onopen: onopen,
      _onmessage: onmessage,
      _onerror: onerror,
      _onclose: onclose
    })
  }

  pub fn send(&self, text: &str) { let _ = self.ws.send_with_str(text); }

  pub fn disconnect(&self) { let _ = self.ws.close(); }

  pub fn poll_events(&self) -> Vec<ConnEvent> {
    let mut events = Vec::new();
    while let Ok(ev) = self.event_rx.try_recv() {
      events.push(ev);
    }
    events
  }
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
