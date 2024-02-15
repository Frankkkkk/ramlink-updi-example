use byteorder::{ByteOrder, LittleEndian};
use serialport::SerialPort;
use std::fmt;

#[path = "crc16.rs"]
mod crc16;

// C.f. JTAGICE mkII Comm. Protocol
// https://ww1.microchip.com/downloads/en/Appnotes/doc2587.pdf

#[derive(Debug)]
#[allow(dead_code)]
pub enum Commands {
    MessageStart = 0x1b,
    Token = 0x0e,

    GetSignOn = 0x01,
    SetParam = 0x02,
    GetParam = 0x03,
    WriteMemory = 0x04,
    ReadMemory = 0x05,
    Go = 0x08,
    Reset = 0x0b,
    SetDeviceDescriptor = 0x0c,
    GetSync = 0x0f,

    EnterProgMode = 0x14,
}

#[derive(Debug)]
pub enum Replies {
    Ok = 0x80,
    Parameter = 0x81,
    Memory = 0x82,
    SignOn = 0x86,
    Failed = 0xA0,
    IllegalMcuState = 0xA5,
    NoTargetPower = 0xAB,
}

impl Replies {
    fn from_code(code: u8) -> Option<Self> {
        match code {
            // XXX is there a way to not duplicate the 0x80⋄Ok rel.?
            0x80 => Some(Replies::Ok),
            0x81 => Some(Replies::Parameter),
            0x82 => Some(Replies::Memory),
            0x86 => Some(Replies::SignOn),
            0xA0 => Some(Replies::Failed),
            0xA5 => Some(Replies::IllegalMcuState),
            0xAB => Some(Replies::NoTargetPower),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct JtagIceMkiiCommand {
    seqno: u16,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct JtagIceMkiiReply {
    pub seqno: u16,
    pub reply: Replies,
    pub data: Vec<u8>,
}

impl JtagIceMkiiCommand {
    fn to_raw(&self) -> Vec<u8> {
        let mut msg = vec![0; 8];

        // Start seq
        msg[0] = Commands::MessageStart as u8;

        // Sequence number
        LittleEndian::write_u16(&mut msg[1..=2], self.seqno);
        LittleEndian::write_u32(&mut msg[3..=6], self.data.len() as u32);
        msg[7] = Commands::Token as u8;

        msg.extend_from_slice(&self.data);

        let crc = crc16::crcsum(&msg);
        let mut crc_array = [0u8; 2];
        LittleEndian::write_u16(&mut crc_array, crc);
        msg.extend_from_slice(&crc_array);

        msg
    }
}
impl JtagIceMkiiReply {
    fn from_raw(raw_data: &[u8]) -> Result<JtagIceMkiiReply, JtagIceMkiiError> {
        if raw_data[0] != Commands::MessageStart as u8 {
            return Err(JtagIceMkiiError::UnmarshallMessageStart);
        }

        let seqno = LittleEndian::read_u16(&raw_data[1..=2]);
        let data_len = LittleEndian::read_u32(&raw_data[3..=6]) as usize + 8 + 2;

        if raw_data[7] != Commands::Token as u8 {
            return Err(JtagIceMkiiError::UnmarshallTokenError);
        }

        let calculated_crc = crc16::crcsum(&raw_data[0..data_len - 2]);
        let recvd_crc = LittleEndian::read_u16(&raw_data[data_len - 2..]);

        if calculated_crc != recvd_crc {
            return Err(JtagIceMkiiError::UnmarshallCrc);
        }

        let useful_data: Vec<u8> = raw_data[8..data_len - 2].to_vec();

        let reply_code = match Replies::from_code(useful_data[0]) {
            Some(x) => x,
            None => return Err(JtagIceMkiiError::UnknownReplyCmnd(useful_data[0])),
        };

        let cmd = JtagIceMkiiReply {
            seqno: seqno,
            reply: reply_code,
            data: useful_data
                .get(1..)
                .map_or_else(|| Vec::new(), |slice| slice.to_vec()),
        };

        Ok(cmd)
    }
}

pub struct JtagIceMkii<'a> {
    pub port: Box<dyn 'a + SerialPort>,
    pub seqno: u16,
}

impl<'a> JtagIceMkii<'_> {
    pub fn new(port: Box<dyn SerialPort>) -> JtagIceMkii<'a> {
        JtagIceMkii { port, seqno: 0 }
    }

    pub fn sign_on(&mut self) -> Result<JtagIceMkiiReply, JtagIceMkiiError> {
        self.send_cmd(&[Commands::GetSignOn as u8]);
        let result = match self.recv_result() {
            Ok(x) => x,
            Err(x) => return Err(x),
        };

        match result.reply {
            Replies::SignOn => return Ok(result),
            _ => return Err(JtagIceMkiiError::NotOk(result.reply)),
        }
    }

    pub fn read_ram_byte(&mut self, mem_addr: u16) -> Result<u8, JtagIceMkiiError> {
        const NUM_BYTES_TO_READ: u16 = 1;

        let mut numbytes_buf = [0u8; 2];
        LittleEndian::write_u16(&mut numbytes_buf, NUM_BYTES_TO_READ);

        let mut addr_buf = [0u8; 2];
        LittleEndian::write_u16(&mut addr_buf, mem_addr);

        self.send_cmd(&[
            Commands::ReadMemory as u8,
            0x20,
            numbytes_buf[0],
            numbytes_buf[1],
            0,
            0,
            addr_buf[0],
            addr_buf[1],
        ]);

        let rcv = match self.recv_result() {
            Ok(x) => x,
            Err(x) => return Err(x),
        };

        self.increase_seqno();

        Ok(rcv.data[0])
    }

    pub fn write_ram_byte(&mut self, mem_addr: u16, value: u8) -> Result<(), JtagIceMkiiError> {
        const NUM_BYTES_TO_WRITE: u16 = 1;

        let mut numbytes_buf = [0u8; 2];
        LittleEndian::write_u16(&mut numbytes_buf, NUM_BYTES_TO_WRITE);

        let mut addr_buf = [0u8; 2];
        LittleEndian::write_u16(&mut addr_buf, mem_addr);

        self.send_cmd(&[
            Commands::WriteMemory as u8,
            0x20, // Mem type SRAM
            numbytes_buf[0],
            numbytes_buf[1],
            0,
            0,
            addr_buf[0],
            addr_buf[1],
            0,
            0,
            value,
        ]);

        //self.increase_seqno();

        match self.recv_result() {
            Ok(_) => Ok(()),
            Err(x) => return Err(x),
        }
    }

    pub fn increase_seqno(&mut self) {
        self.seqno += 1;
    }

    pub fn send_cmd(&mut self, data: &[u8]) {
        let cmd = JtagIceMkiiCommand {
            seqno: self.seqno,
            data: data.to_vec(),
        };

        let raw_cmd = cmd.to_raw();
        //        println!("will send: {:02x?} => {:02x?} ?", cmd, raw_cmd);
        self.port.write(&raw_cmd).unwrap(); // XXX Return an error
    }

    pub fn recv_result(&mut self) -> Result<JtagIceMkiiReply, JtagIceMkiiError> {
        let mut raw_data: Vec<u8> = vec![0; 0];
        let mut total_data_length: usize = 6; // read at least 6 char (that contain the size)

        while raw_data.len() < total_data_length {
            let mut buf: Vec<u8> = vec![0; 2000];
            let res = self.port.read(buf.as_mut_slice());
            match res {
                Ok(datalen) => {
                    buf.truncate(datalen);
                    raw_data.append(&mut buf);

                    total_data_length = LittleEndian::read_u32(&raw_data[3..=6]) as usize + 6;
                    raw_data.truncate(total_data_length + 6);
                }
                Err(_err) => {
                    return Err(JtagIceMkiiError::NoReply);
                }
            }
        }

        let reply = JtagIceMkiiReply::from_raw(&raw_data).unwrap();
        if reply.seqno != self.seqno {
            return Err(JtagIceMkiiError::DifferentSeqNo);
        }

        // Now that we have received data, we can increase the seqno
        //self.increase_seqno();

        return Ok(reply);
    }
}

pub const SET_DEV_DESCRIPTOR: &[u8] = &[
    Commands::SetDeviceDescriptor as u8,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    0x40,
    00,
    0x20,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    10,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    00,
    0x40,
    00,
    00,
    00,
    00,
    0x80,
    0x07,
    01,
    02,
    00,
    00,
    00,
    00,
    00,
    00,
    0x3f,
    00,
];

//#[derive(Copy, Clone, PartialEq, Eq)]
pub enum JtagIceMkiiError {
    NoReply,
    DifferentSeqNo,
    NotOk(Replies),
    UnmarshallTokenError,
    UnmarshallMessageStart,
    UnmarshallCrc,
    UnknownReplyCmnd(u8),
}

#[cfg(feature = "std")]
impl std::error::Error for JtagIceMkiiError {}

impl fmt::Debug for JtagIceMkiiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JtagIceMkiiError::NoReply => f.pad("No Reply"),
            JtagIceMkiiError::DifferentSeqNo => f.pad("Seqno ≠"),
            JtagIceMkiiError::NotOk(rep) => {
                f.pad(&format!("Reply code was not 'OK', but {:?} instead", rep))
            }
            JtagIceMkiiError::UnmarshallTokenError => f.pad("Unmarshall token error"),
            JtagIceMkiiError::UnmarshallCrc => f.pad("Unmarshall CRC check failed"),
            JtagIceMkiiError::UnknownReplyCmnd(code) => {
                f.pad(&format!("Unknown reply command code {:02x}", code))
            }
            JtagIceMkiiError::UnmarshallMessageStart => {
                f.pad("Reply message doesn't start with MessageStart")
            }
        }
    }
}

/*
impl fmt::Display for JtagIceMkiiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JtagIceMkiiError::NoReply => "No reply".fmt(f),
            JtagIceMkiiError::DifferentSeqNo => "Different sequence number".fmt(f),
            JtagIceMkiiError::UnmarshallTokenError => "Unmarshall token error".fmt(f),
            JtagIceMkiiError::UnmarshallCrc => "Unmarshall CRC check failed".fmt(f),
            JtagIceMkiiError::UnknownReplyCmnd => "Unknown reply command code".fmt(f),
            JtagIceMkiiError::UnmarshallMessageStart => {
                "Reply message doesn't start with MessageStart".fmt(f)
            }
        }
    }
}

*/
