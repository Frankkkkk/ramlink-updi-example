use std::time::Duration;

use byteorder::{ByteOrder, LittleEndian};
use serialport::{SerialPort, SerialPortInfo};

mod crc16;
#[warn(non_camel_case_types)]
enum Commands {
    MESSAGE_START = 0x1b,
    TOKEN = 0x0e,

    CMND_GET_SIGN_ON = 0x01,
}

struct JtagIceMkiiCommand {
    seqno: u16,
    data: [u8],
}

impl JtagIceMkiiCommand {
    fn to_raw(&self) -> Vec<u8> {
        let mut msg = vec![0; 8];

        // Start seq
        msg[0] = Commands::MESSAGE_START as u8;

        // Sequence number
        LittleEndian::write_u16(&mut msg[1..=2], self.seqno);
        LittleEndian::write_u32(&mut msg[3..=6], self.data.len() as u32);
        msg[7] = Commands::TOKEN as u8;

        msg.extend_from_slice(&self.data);

        let crc = crc16::crcsum(&msg);
        let mut crc_array = [0u8; 2];
        LittleEndian::write_u16(&mut crc_array, crc);
        msg.extend_from_slice(&crc_array);

        msg
    }
}

struct JtagIceMkii {
    port: Box<dyn SerialPort>,
    seqno: u16,
}

impl JtagIceMkii {
    fn new(port: Box<dyn SerialPort>) -> JtagIceMkii {
        JtagIceMkii { port, seqno: 0 }
    }
    fn send_cmd(&self, data: &[u8]) {
        let mut msg = vec![0; 8];

        // Start seq
        msg[0] = Commands::MESSAGE_START as u8;

        // Sequence number
        LittleEndian::write_u16(&mut msg[1..=2], self.seqno);
        LittleEndian::write_u32(&mut msg[3..=6], data.len() as u32);
        msg[7] = Commands::TOKEN as u8;

        msg.extend_from_slice(data);

        let crc = crc16::crcsum(&msg);
        let mut crc_array = [0u8; 2];
        LittleEndian::write_u16(&mut crc_array, crc);
        msg.extend_from_slice(&crc_array);

        println!("{:02x?}", msg)
    }
}

fn main() {
    let mut port = serialport::new("/dev/ttyUSB0", 19200)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(10))
        .open()
        .expect("Failed to open port");

    let dgr = JtagIceMkii::new(port);

    dgr.send_cmd(&[Commands::CMND_GET_SIGN_ON as u8]);

    //#   let output = "This is a test. This is only a test.".as_bytes();
    //#   port.write(output).expect("Write failed!");

    //    let mut serial_buf: Vec<u8> = vec![0; 32];
    //    port.read(serial_buf.as_mut_slice())
    //        .expect("Found no data!");
    //    println!("Hello, world!");
}
