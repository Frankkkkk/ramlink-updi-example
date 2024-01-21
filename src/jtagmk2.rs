use byteorder::{ByteOrder, LittleEndian};
use serialport::SerialPort;
use std::fmt;

#[path = "crc16.rs"]
mod crc16;

#[derive(Debug)]
pub enum Commands {
    MessageStart = 0x1b,
    Token = 0x0e,

    GetSignOn = 0x01,
    SetParam = 0x02,
    GetParam = 0x03,
    ReadMemory = 0x05,
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
    NoTargetPower = 0xAB,
    NoTargetPower1 = 0xC1,
    NoTargetPower2 = 0xC2,
    NoTargetPower3 = 0xC3,
    NoTargetPower4 = 0xC4,
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
            0xAB => Some(Replies::NoTargetPower),
            0xC1 => Some(Replies::NoTargetPower1),
            0xC2 => Some(Replies::NoTargetPower2),
            0xC3 => Some(Replies::NoTargetPower3),
            0xC4 => Some(Replies::NoTargetPower4),

            _ => None,
        }
    }
}

#[derive(Debug)]
struct MyError;

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Custom error message")
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
    fn from_raw(raw_data: &[u8]) -> Result<JtagIceMkiiReply, String> {
        if raw_data[0] != Commands::MessageStart as u8 {
            return Err(format!("Fucked up message start"));
        }

        let seqno = LittleEndian::read_u16(&raw_data[1..=2]);
        let data_len = LittleEndian::read_u32(&raw_data[3..=6]) as usize + 8 + 2;

        if raw_data[7] != Commands::Token as u8 {
            return Err(format!("Fucked up token"));
        }

        let calculated_crc = crc16::crcsum(&raw_data[0..data_len - 2]);
        let recvd_crc = LittleEndian::read_u16(&raw_data[data_len - 2..]);

        if calculated_crc != recvd_crc {
            return Err(format!(
                "CRC is fucked up. Recvd {}, calculated: {}",
                recvd_crc, calculated_crc,
            ));
        }

        let useful_data: Vec<u8> = raw_data[8..data_len - 2].to_vec();

        let cmd = JtagIceMkiiReply {
            seqno: seqno,
            reply: Replies::from_code(useful_data[0])
                .expect(&format!("Unknown code {:02x}", useful_data[0])),
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
    pub fn increase_seqno(&mut self) {
        self.seqno += 1;
    }
    pub fn send_cmd(&mut self, data: &[u8]) {
        let cmd = JtagIceMkiiCommand {
            seqno: self.seqno,
            data: data.to_vec(),
        };

        let raw_cmd = cmd.to_raw();
        //println!("will send: {:02x?} => {:02x?}", cmd, raw_cmd);
        self.port.write(&raw_cmd).unwrap(); // XXX Return an error
    }
    pub fn recv_result(&mut self) -> Result<JtagIceMkiiReply, String> {
        let mut raw_data: Vec<u8> = vec![0; 0];
        let mut total_data_length: usize = 6; // read at least 6 char (that contain the size)
                                              //

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
                    //println!("Didn't receive shit");
                    return Err(format!("Didnot recieve shit"));
                }
            }
        }

        //println!("RAW DATA: {:02x?}", raw_data);

        let reply = JtagIceMkiiReply::from_raw(&raw_data).unwrap();
        //println!("Received: {:02x?}", reply);
        if reply.seqno != self.seqno {
            //println!("Seqno not the same !");
            return Err(format!("SEQNO is not the same"));
        }
        //println!("CMD: {:02x?}", cmd);
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
