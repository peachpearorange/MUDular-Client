use std::collections::HashMap;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

use crate::telnet::*;
use crate::protocol::mssp;

pub async fn probe_mssp(host: &str, port: u16) -> Option<HashMap<String, String>> {
    let addr = format!("{host}:{port}");
    let stream = timeout(Duration::from_secs(3), TcpStream::connect(&addr))
        .await.ok()?.ok()?;

    let (mut reader, mut writer) = stream.into_split();
    let mut parser = TelnetParser::new();
    let mut buf = [0u8; 4096];
    let mut result = None;

    let _ = timeout(Duration::from_secs(5), async {
        loop {
            match reader.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    for event in parser.parse(&buf[..n]) {
                        match event {
                            TelnetEvent::Negotiate(WILL, OPT_MSSP) => {
                                let _ = writer.write_all(&TelnetParser::build_do(OPT_MSSP)).await;
                            }
                            TelnetEvent::Subnegotiation(OPT_MSSP, data) => {
                                result = Some(mssp::parse_mssp(&data));
                            }
                            TelnetEvent::Negotiate(WILL, opt) => {
                                let _ = writer.write_all(&TelnetParser::build_dont(opt)).await;
                            }
                            TelnetEvent::Negotiate(DO, opt) => {
                                let _ = writer.write_all(&TelnetParser::build_wont(opt)).await;
                            }
                            _ => {}
                        }
                    }
                    if result.is_some() { break; }
                }
            }
        }
    }).await;

    result
}
