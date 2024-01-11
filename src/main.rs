use byteorder::{ByteOrder, LittleEndian};
use serialport::SerialPort;
use std::{fmt, time::Duration};

mod crc16;
enum Commands {
    MessageStart = 0x1b,
    Token = 0x0e,

    GetSignOn = 0x01,
}

#[derive(Debug)]
struct MyError;

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Custom error message")
    }
}

#[derive(Debug)]
struct JtagIceMkiiCommand {
    seqno: u16,
    data: Vec<u8>,
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

    fn from_raw(raw_data: &[u8]) -> Result<JtagIceMkiiCommand, String> {
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

        let mut useful_data: Vec<u8> = raw_data[8..].to_vec();

        let cmd = JtagIceMkiiCommand {
            seqno: seqno,
            data: useful_data,
        };

        Ok(cmd)
    }
}

struct JtagIceMkii<'a> {
    port: Box<dyn 'a + SerialPort>,
    seqno: u16,
}

impl<'a> JtagIceMkii<'_> {
    fn new(port: Box<dyn SerialPort>) -> JtagIceMkii<'a> {
        JtagIceMkii { port, seqno: 0 }
    }
    fn send_cmd(&mut self, data: &[u8]) {
        let cmd = JtagIceMkiiCommand {
            seqno: self.seqno,
            data: data.to_vec(),
        };

        let raw_cmd = cmd.to_raw();
        self.port.write(&raw_cmd); // XXX Return an error
    }
    fn recv_result(&mut self) -> Result<JtagIceMkiiCommand, String> {
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
                    println!("Didn't receive shit");
                    return Err(format!("Didnot recieve shit"));
                }
            }
        }

        let cmd = JtagIceMkiiCommand::from_raw(&raw_data).unwrap();
        return Ok(cmd);
    }
}

fn main() {
    let port = serialport::new("/dev/ttyUSB0", 19200)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_secs(8))
        .open()
        .expect("Failed to open port");

    let mut dgr = JtagIceMkii::new(port);

    dgr.send_cmd(&[Commands::GetSignOn as u8]);
    dgr.recv_result();
    dgr.send_cmd(&[Commands::GetSignOn as u8]);
    dgr.recv_result();
}
