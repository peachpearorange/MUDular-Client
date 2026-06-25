pub const IAC: u8 = 255;
pub const DONT: u8 = 254;
pub const DO: u8 = 253;
pub const WONT: u8 = 252;
pub const WILL: u8 = 251;
pub const SB: u8 = 250;
pub const SE: u8 = 240;
pub const GA: u8 = 249;

pub const OPT_ECHO: u8 = 1;
pub const OPT_SGA: u8 = 3;
pub const OPT_TTYPE: u8 = 24;
pub const OPT_NAWS: u8 = 31;
pub const OPT_MCCP2: u8 = 86;
pub const OPT_MSSP: u8 = 70;
pub const OPT_MSDP: u8 = 69;
pub const OPT_GMCP: u8 = 201;

#[derive(Debug, Clone)]
pub enum TelnetEvent {
    Data(Vec<u8>),
    Negotiate(u8, u8),
    Subnegotiation(u8, Vec<u8>),
    GoAhead,
}

enum State {
    Data,
    Iac,
    Negotiate(u8),
    Subneg(u8, Vec<u8>),
    SubnegIac(u8, Vec<u8>),
}

pub struct TelnetParser {
    state: State,
}

impl TelnetParser {
    pub fn new() -> Self {
        Self { state: State::Data }
    }

    pub fn parse(&mut self, input: &[u8]) -> Vec<TelnetEvent> {
        let mut events = Vec::new();
        let mut data_buf = Vec::new();

        for &byte in input {
            self.state = match std::mem::replace(&mut self.state, State::Data) {
                State::Data => {
                    if byte == IAC {
                        if !data_buf.is_empty() {
                            events.push(TelnetEvent::Data(std::mem::take(&mut data_buf)));
                        }
                        State::Iac
                    } else {
                        data_buf.push(byte);
                        State::Data
                    }
                }
                State::Iac => match byte {
                    IAC => {
                        data_buf.push(IAC);
                        State::Data
                    }
                    DO | DONT | WILL | WONT => State::Negotiate(byte),
                    SB => State::Subneg(0, Vec::new()),
                    GA => {
                        events.push(TelnetEvent::GoAhead);
                        State::Data
                    }
                    _ => State::Data,
                },
                State::Negotiate(cmd) => {
                    events.push(TelnetEvent::Negotiate(cmd, byte));
                    State::Data
                }
                State::Subneg(opt, buf) => {
                    if opt == 0 {
                        State::Subneg(byte, buf)
                    } else if byte == IAC {
                        State::SubnegIac(opt, buf)
                    } else {
                        let mut buf = buf;
                        buf.push(byte);
                        State::Subneg(opt, buf)
                    }
                }
                State::SubnegIac(opt, buf) => {
                    if byte == SE {
                        events.push(TelnetEvent::Subnegotiation(opt, buf));
                        State::Data
                    } else if byte == IAC {
                        let mut buf = buf;
                        buf.push(IAC);
                        State::Subneg(opt, buf)
                    } else {
                        State::Subneg(opt, buf)
                    }
                }
            };
        }

        if !data_buf.is_empty() {
            events.push(TelnetEvent::Data(data_buf));
        }

        events
    }

    pub fn build_will(opt: u8) -> Vec<u8> {
        vec![IAC, WILL, opt]
    }

    pub fn build_wont(opt: u8) -> Vec<u8> {
        vec![IAC, WONT, opt]
    }

    pub fn build_do(opt: u8) -> Vec<u8> {
        vec![IAC, DO, opt]
    }

    pub fn build_dont(opt: u8) -> Vec<u8> {
        vec![IAC, DONT, opt]
    }

    pub fn build_subneg(opt: u8, data: &[u8]) -> Vec<u8> {
        let mut buf = vec![IAC, SB, opt];
        for &b in data {
            buf.push(b);
            if b == IAC {
                buf.push(IAC);
            }
        }
        buf.push(IAC);
        buf.push(SE);
        buf
    }
}

