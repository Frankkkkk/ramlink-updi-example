use std::{error::Error, fmt, os::linux::raw, sync::Arc, time::Duration};

use byteorder::{ByteOrder, LittleEndian};
use serialport::SerialPort;

mod crc16;
enum Commands {
    #[warn(non_camel_case_types)]
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
struct JtagIceMkiiCommand<'a> {
    seqno: u16,
    data: &'a [u8],
}

impl JtagIceMkiiCommand<'_> {
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

    fn from_raw(raw_data: &'_ [u8]) -> Result<JtagIceMkiiCommand<'_>, String> {
        println!("\n\n\n");
        println!("MY MESSAGE IS {:02x?}", raw_data);

        if raw_data[0] != Commands::MessageStart as u8 {
            return Err(format!("Fucked up message start"));
        }

        let seqno = LittleEndian::read_u16(&raw_data[1..=2]);
        let data_len = LittleEndian::read_u32(&raw_data[3..=6]) as usize + 8 + 2;
        println!("DATA LEN IS {}", data_len);

        if raw_data[7] != Commands::Token as u8 {
            return Err(format!("Fucked up token"));
        }

        let calculated_crc = crc16::crcsum(&raw_data[0..data_len - 2]);
        println!(
            "WILL CRCSUM {:02x?} -> {}",
            &raw_data[0..data_len],
            calculated_crc
        );
        let recvd_crc = LittleEndian::read_u16(&raw_data[data_len - 2..]);

        if calculated_crc != recvd_crc {
            return Err(format!(
                "CRC is fucked up. Recvd {}, calculated: {}",
                recvd_crc, calculated_crc,
            ));
        }

        let cmd = JtagIceMkiiCommand {
            seqno: seqno,
            data: &raw_data[8..],
        };

        println!("CMD IS {:?}", cmd);

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
            data,
        };

        let raw_cmd = cmd.to_raw();
        println!(">>> SND: {:02x?}", raw_cmd);
        self.port.write(&raw_cmd);
    }
    fn recv_result(&mut self) -> Result<Vec<u8>, String> {
        let mut raw_data: Vec<u8> = vec![0; 0];
        let mut total_data_length: usize = 6; // read at least 6 char (that contain the size)
                                              //

        while raw_data.len() < total_data_length {
            let mut buf: Vec<u8> = vec![0; 2000];
            let res = self.port.read(buf.as_mut_slice());
            match res {
                Ok(datalen) => {
                    buf.truncate(datalen);
                    println!("RAW DATA IN (len {datalen}): {:02x?}", buf);
                    raw_data.append(&mut buf);
                    println!("BUFFER IS NOW {:02x?} ({})", raw_data, raw_data.len());

                    total_data_length = LittleEndian::read_u32(&raw_data[3..=6]) as usize + 6;
                    raw_data.truncate(total_data_length + 6);
                    println!("New len: {total_data_length}");
                    println!("New raw_data({}): {:02x?}", raw_data.len(), raw_data);
                }
                Err(err) => {
                    println!("Didn't receive shit");
                    return Err(format!("Didnot recieve shit"));
                }
            }
        }

        println!("FINAL raw_data({}): {:02x?}", raw_data.len(), raw_data);
        let cmd = JtagIceMkiiCommand::from_raw(&raw_data);
        println!("CMD: {:?}", cmd);
        return Ok(raw_data);
    }
}

fn main() {
    let mut port = serialport::new("/dev/ttyUSB0", 19200)
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
