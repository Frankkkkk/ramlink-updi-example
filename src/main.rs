use core::num;
use std::{io, time::Duration};

use byteorder::{ByteOrder, LittleEndian};

//use crate::jtagmk2::JtagIceMkiiCommand;

mod jtagmk2;

fn main() {
    let port = serialport::new("/dev/ttyUSB0", 19200)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_secs(8))
        .open()
        .expect("Failed to open port");

    let mut dgr = jtagmk2::JtagIceMkii::new(port);

    dgr.sign_on();
    dgr.sign_on();

    //Set bd rate to 115200
    println!(">>> Will set baud rate");
    dgr.send_cmd(&[jtagmk2::Commands::SetParam as u8, 0x05, 0x07]);
    dgr.recv_result();
    dgr.increase_seqno();
    dgr.port.set_baud_rate(115200);

    // XXX Set device descriptor
    println!("Will set device descriptor again ?!");
    dgr.send_cmd(&[jtagmk2::Commands::SetDeviceDescriptor as u8, 0x05, 0x07]);
    dgr.recv_result();
    dgr.increase_seqno();
    dgr.port.set_baud_rate(115200);

    //*/
    //for mem_addr in 0x3f00..0x3fff {
    //for mem_addr in 0x8000..0x8010 {
    loop {
        let mut ram: Vec<u8> = vec![];
        for mem_addr in 0x3f00..0x3f0f {
            let byte = dgr.read_ram_byte(mem_addr);
            match byte {
                Ok(val) => ram.push(val),
                Err(_) => ram.push(0),
            }
        }

        for chunk in ram.chunks(10) {
            println!("{:02x?}", chunk)
        }

        dgr.write_ram_byte(0x3f00, 0x43)
            .expect("Couldn't write ram");

        println!("Continue loop ?");
        let mut _buffer = String::new();
        io::stdin().read_line(&mut _buffer);
    }
}
