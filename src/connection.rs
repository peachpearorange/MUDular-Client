use std::sync::mpsc;
use log::{debug, info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::protocol::gmcp;
use crate::protocol::mccp::MccpDecompressor;
use crate::protocol::mssp;
use crate::telnet::*;

pub enum ConnEvent {
    Connected,
    Data(String),
    GmcpReceived(String, serde_json::Value),
    MsspReceived(std::collections::HashMap<String, String>),
    MsdpReceived(serde_json::Value),
    Disconnected(String),
    Error(String),
}

pub enum ConnCommand {
    Send(String),
    SendGmcp(String, serde_json::Value),
    SendMsdpReport(Vec<String>),
    SendMsdpSend(Vec<String>),
    SendMsdpList(String),
    Disconnect,
}

pub struct Connection {
    pub event_rx: mpsc::Receiver<ConnEvent>,
    pub cmd_tx: mpsc::Sender<ConnCommand>,
}

impl Connection {
    pub fn connect(host: String, port: u16, tls: bool, runtime: &tokio::runtime::Runtime) -> Self {
        let (event_tx, event_rx) = mpsc::channel();
        let (cmd_tx, cmd_rx) = mpsc::channel();

        runtime.spawn(connection_task(host, port, tls, event_tx, cmd_rx));

        Self { event_rx, cmd_tx }
    }

    pub fn send(&self, text: &str) {
        let _ = self.cmd_tx.send(ConnCommand::Send(text.to_string()));
    }

    pub fn send_gmcp(&self, package: &str, data: &serde_json::Value) {
        let _ = self.cmd_tx.send(ConnCommand::SendGmcp(package.to_string(), data.clone()));
    }

    pub fn send_msdp_report(&self, vars: Vec<String>) {
        let _ = self.cmd_tx.send(ConnCommand::SendMsdpReport(vars));
    }

    pub fn send_msdp_send(&self, vars: Vec<String>) {
        let _ = self.cmd_tx.send(ConnCommand::SendMsdpSend(vars));
    }

    pub fn send_msdp_list(&self, what: String) {
        let _ = self.cmd_tx.send(ConnCommand::SendMsdpList(what));
    }

    pub fn disconnect(&self) {
        let _ = self.cmd_tx.send(ConnCommand::Disconnect);
    }

    pub fn poll_events(&self) -> Vec<ConnEvent> {
        let mut events = Vec::new();
        while let Ok(ev) = self.event_rx.try_recv() {
            events.push(ev);
        }
        events
    }
}

async fn connection_task(
    host: String,
    port: u16,
    tls: bool,
    event_tx: mpsc::Sender<ConnEvent>,
    cmd_rx: mpsc::Receiver<ConnCommand>,
) {
    info!("Connecting to {host}:{port} (tls={tls})");
    let tcp_stream = match TcpStream::connect(format!("{host}:{port}")).await {
        Ok(s) => s,
        Err(e) => {
            warn!("Connection failed: {e}");
            let _ = event_tx.send(ConnEvent::Error(format!("Connection failed: {e}")));
            return;
        }
    };

    info!("TCP connected to {host}:{port}");

    if tls {
        let root_store = rustls::RootCertStore::from_iter(
            webpki_roots::TLS_SERVER_ROOTS.iter().cloned(),
        );
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(std::sync::Arc::new(config));
        let server_name = match rustls::pki_types::ServerName::try_from(host.as_str()) {
            Ok(name) => name.to_owned(),
            Err(e) => {
                let _ = event_tx.send(ConnEvent::Error(format!("Invalid TLS hostname: {e}")));
                return;
            }
        };
        match connector.connect(server_name, tcp_stream).await {
            Ok(tls_stream) => {
                info!("TLS handshake complete");
                let _ = event_tx.send(ConnEvent::Connected);
                run_connection(tls_stream, event_tx, cmd_rx).await;
            }
            Err(e) => {
                let _ = event_tx.send(ConnEvent::Error(format!("TLS handshake failed: {e}")));
            }
        }
    } else {
        let _ = event_tx.send(ConnEvent::Connected);
        run_connection(tcp_stream, event_tx, cmd_rx).await;
    }
}

async fn run_connection<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static>(
    stream: S,
    event_tx: mpsc::Sender<ConnEvent>,
    cmd_rx: mpsc::Receiver<ConnCommand>,
) {
    let (mut reader, mut writer) = tokio::io::split(stream);
    let mut parser = TelnetParser::new();
    let mut mccp = MccpDecompressor::new();
    let mut mccp_pending = false;
    let mut gmcp_enabled = false;
    let mut partial_line = String::new();

    let (write_tx, mut write_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(64);

    let write_handle = tokio::spawn(async move {
        while let Some(data) = write_rx.recv().await {
            if writer.write_all(&data).await.is_err() {
                break;
            }
        }
    });

    let cmd_write_tx = write_tx.clone();
    let cmd_handle = tokio::spawn(async move {
        loop {
            match tokio::task::block_in_place(|| cmd_rx.recv()) {
                Ok(ConnCommand::Send(text)) => {
                    let mut data = text.into_bytes();
                    data.extend_from_slice(b"\r\n");
                    if cmd_write_tx.send(data).await.is_err() {
                        break;
                    }
                }
                Ok(ConnCommand::SendGmcp(package, value)) => {
                    let payload = gmcp::encode_gmcp(&package, &value);
                    let data = TelnetParser::build_subneg(OPT_GMCP, &payload);
                    if cmd_write_tx.send(data).await.is_err() {
                        break;
                    }
                }
                Ok(ConnCommand::SendMsdpReport(vars)) => {
                    let payload = crate::protocol::msdp::encode_msdp_report(&vars);
                    let data = TelnetParser::build_subneg(OPT_MSDP, &payload);
                    if cmd_write_tx.send(data).await.is_err() {
                        break;
                    }
                }
                Ok(ConnCommand::SendMsdpSend(vars)) => {
                    let payload = crate::protocol::msdp::encode_msdp_send(&vars);
                    let data = TelnetParser::build_subneg(OPT_MSDP, &payload);
                    if cmd_write_tx.send(data).await.is_err() {
                        break;
                    }
                }
                Ok(ConnCommand::SendMsdpList(what)) => {
                    let payload = crate::protocol::msdp::encode_msdp_var("LIST", &what);
                    let data = TelnetParser::build_subneg(OPT_MSDP, &payload);
                    if cmd_write_tx.send(data).await.is_err() {
                        break;
                    }
                }
                Ok(ConnCommand::Disconnect) | Err(_) => break,
            }
        }
    });

    let mut buf = [0u8; 4096];
    loop {
        match reader.read(&mut buf).await {
            Ok(0) => {
                let _ = event_tx.send(ConnEvent::Disconnected("Server closed connection".into()));
                break;
            }
            Ok(n) => {
                let raw = &buf[..n];
                let data = if mccp.is_active() {
                    match mccp.decompress_data(raw) {
                        Ok(d) => d,
                        Err(e) => {
                            let _ = event_tx.send(ConnEvent::Error(e));
                            break;
                        }
                    }
                } else {
                    raw.to_vec()
                };

                let events = parser.parse(&data);
                for event in events {
                    match event {
                        TelnetEvent::Data(bytes) => {
                            let text = String::from_utf8_lossy(&bytes);
                            partial_line.push_str(&text);

                            while let Some(newline_pos) = partial_line.find('\n') {
                                let line = partial_line[..newline_pos].trim_end_matches('\r').to_string();
                                partial_line = partial_line[newline_pos + 1..].to_string();
                                let _ = event_tx.send(ConnEvent::Data(line));
                            }
                        }
                        TelnetEvent::Negotiate(cmd, opt) => {
                            debug!("Telnet negotiate: cmd={cmd} opt={opt}");
                            let response = handle_negotiation(cmd, opt, &mut mccp_pending, &mut gmcp_enabled);
                            for resp in response {
                                let _ = write_tx.send(resp).await;
                            }
                        }
                        TelnetEvent::Subnegotiation(opt, sub_data) => {
                            debug!("Telnet subneg: opt={opt} len={}", sub_data.len());
                            match opt {
                                OPT_MCCP2 => {
                                    info!("MCCP2 activated");
                                    mccp.activate();
                                    mccp_pending = false;
                                }
                                OPT_GMCP => {
                                    if let Some(msg) = gmcp::parse_gmcp(&sub_data) {
                                        let _ = event_tx.send(ConnEvent::GmcpReceived(msg.package, msg.data));
                                    }
                                }
                                OPT_MSSP => {
                                    let info = mssp::parse_mssp(&sub_data);
                                    let _ = event_tx.send(ConnEvent::MsspReceived(info));
                                }
                                OPT_MSDP => {
                                    let data = crate::protocol::msdp::parse_msdp(&sub_data);
                                    let _ = event_tx.send(ConnEvent::MsdpReceived(data));
                                }
                                OPT_TTYPE => {
                                    if sub_data.first() == Some(&1) {
                                        let mut resp = vec![0u8];
                                        resp.extend_from_slice(b"MUDular");
                                        let _ = write_tx.send(TelnetParser::build_subneg(OPT_TTYPE, &resp)).await;
                                    }
                                }
                                _ => {}
                            }
                        }
                        TelnetEvent::GoAhead => {
                            if !partial_line.is_empty() {
                                let line = std::mem::take(&mut partial_line);
                                let _ = event_tx.send(ConnEvent::Data(line));
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let _ = event_tx.send(ConnEvent::Disconnected(format!("Read error: {e}")));
                break;
            }
        }
    }

    drop(write_tx);
    let _ = write_handle.await;
    let _ = cmd_handle.await;
}

fn handle_negotiation(cmd: u8, opt: u8, mccp_pending: &mut bool, gmcp_enabled: &mut bool) -> Vec<Vec<u8>> {
    let mut responses = Vec::new();
    match cmd {
        WILL => match opt {
            OPT_MCCP2 => {
                *mccp_pending = true;
                responses.push(TelnetParser::build_do(opt));
            }
            OPT_MSSP => responses.push(TelnetParser::build_do(opt)),
            OPT_MSDP => {
                responses.push(TelnetParser::build_do(opt));
                let list_query = crate::protocol::msdp::encode_msdp_var("LIST", "REPORTABLE_VARIABLES");
                responses.push(TelnetParser::build_subneg(OPT_MSDP, &list_query));
            }
            OPT_GMCP => {
                *gmcp_enabled = true;
                responses.push(TelnetParser::build_do(opt));
                let hello = gmcp::encode_gmcp(
                    "Core.Hello",
                    &serde_json::json!({"client": "MUDular", "version": "0.1.0"}),
                );
                responses.push(TelnetParser::build_subneg(OPT_GMCP, &hello));
            }
            OPT_SGA | OPT_ECHO => responses.push(TelnetParser::build_do(opt)),
            _ => responses.push(TelnetParser::build_dont(opt)),
        },
        DO => match opt {
            OPT_TTYPE | OPT_NAWS | OPT_SGA => responses.push(TelnetParser::build_will(opt)),
            OPT_GMCP => {
                *gmcp_enabled = true;
                responses.push(TelnetParser::build_will(opt));
            }
            _ => responses.push(TelnetParser::build_wont(opt)),
        },
        WONT => responses.push(TelnetParser::build_dont(opt)),
        DONT => responses.push(TelnetParser::build_wont(opt)),
        _ => {}
    }
    responses
}
