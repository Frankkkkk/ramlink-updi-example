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

    dgr.send_cmd(&[jtagmk2::Commands::GetSignOn as u8]);
    dgr.recv_result();

    dgr.send_cmd(&[jtagmk2::Commands::GetSignOn as u8]);
    dgr.recv_result();
    dgr.increase_seqno();

    println!(">>> Will set param");
    dgr.send_cmd(&[jtagmk2::Commands::SetParam as u8, 0x03, 0x06]);
    dgr.recv_result();
    dgr.increase_seqno();

    println!(">>> Will get sync");
    dgr.send_cmd(&[jtagmk2::Commands::GetSync as u8]);
    let a = dgr.recv_result();
    dgr.increase_seqno();

    println!(">>> Will get param 0x01");
    dgr.send_cmd(&[jtagmk2::Commands::GetParam as u8, 0x01]);
    let a = dgr.recv_result();
    dgr.increase_seqno();

    println!(">>> Will get param 0x02");
    dgr.send_cmd(&[jtagmk2::Commands::GetParam as u8, 0x02]);
    let a = dgr.recv_result();
    dgr.increase_seqno();

    //Set bd rate to 115200
    println!(">>> Will set baud rate");
    dgr.send_cmd(&[jtagmk2::Commands::SetParam as u8, 0x05, 0x07]);
    dgr.recv_result();
    dgr.increase_seqno();
    dgr.port.set_baud_rate(115200);

    // AT this point we have the device descriptor
    println!("Will set device descriptor");
    dgr.send_cmd(jtagmk2::SET_DEV_DESCRIPTOR);
    let a = dgr.recv_result();
    dgr.increase_seqno();

    // XXX Set device descriptor
    dgr.send_cmd(&[jtagmk2::Commands::SetDeviceDescriptor as u8, 0x05, 0x07]);
    dgr.recv_result();
    dgr.increase_seqno();
    dgr.port.set_baud_rate(115200);

    //println!(">>> Will reset");
    //dgr.send_cmd(&[jtagmk2::Commands::Reset as u8, 0x01]);
    //let a = dgr.recv_result();
    //dgr.increase_seqno();

    println!(">>> Will enter progmode");
    dgr.send_cmd(&[jtagmk2::Commands::EnterProgMode as u8]);
    let a = dgr.recv_result();

    println!(">>> Will enter progmode (again)");
    dgr.send_cmd(&[jtagmk2::Commands::EnterProgMode as u8]);
    let a = dgr.recv_result();

    // Will send read mem
    println!(">>> Will read mem (signature 1)");
    dgr.send_cmd(&[
        jtagmk2::Commands::ReadMemory as u8,
        0xb4,
        0x01,
        0,
        0,
        0,
        0,
        0x11,
        0,
        0,
    ]);
    let a = dgr.recv_result();

    println!(">>> Will read mem (signature 2)");
    dgr.send_cmd(&[
        jtagmk2::Commands::ReadMemory as u8,
        0xb4,
        0x01,
        0,
        0,
        0,
        0x01,
        0x11,
        0,
        0,
    ]);
    let a = dgr.recv_result();

    println!(">>> Will read mem (signature 3)");
    dgr.send_cmd(&[
        jtagmk2::Commands::ReadMemory as u8,
        0xb4,
        0x01,
        0,
        0,
        0,
        0x02,
        0x11,
        0,
        0,
    ]);
    let a = dgr.recv_result();

    /* Flash looks like so:
    18465     memory "flash"
    18466         size               = 4096;
    18467         page_size          = 64;
    18468         offset             = 0x8000;
    18469         readsize           = 256;

    0000  19 c0 33 c0 32 c0 31 c0  30 c0 2f c0 2e c0 2d c0  |..3.2.1.0./...-.|
    0010  2c c0 2b c0 2a c0 29 c0  28 c0 27 c0 26 c0 25 c0  |,.+.*.).(.'.&.%.|

         */

    //for mem_addr in 0x3f00..0x3fff {
    //for mem_addr in 0x8000..0x8010 {
    loop {
        let mut ram: Vec<u8> = vec![];
        for mem_addr in 0x3f00..0x3f0f {
            let mut numbytes_buf = [0u8; 2];
            LittleEndian::write_u16(&mut numbytes_buf, 1);

            let mut addr_buf = [0u8; 2];
            LittleEndian::write_u16(&mut addr_buf, mem_addr);
            dgr.send_cmd(&[
                jtagmk2::Commands::ReadMemory as u8,
                0,
                numbytes_buf[0],
                numbytes_buf[1],
                0,
                0,
                addr_buf[0],
                addr_buf[1],
            ]);

            let rcv = dgr.recv_result().unwrap();
            //println!("ADDR {mem_addr:02x} = {:02x?}", rcv.data[0]);
            ram.push(rcv.data[0]);
        }

        for chunk in ram.chunks(10) {
            println!("{:02x?}", chunk)
        }
        let mut _buffer = String::new();
        io::stdin().read_line(&mut _buffer);
    }
}
