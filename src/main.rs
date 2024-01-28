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

    /*
    dgr.send_cmd(&[jtagmk2::Commands::GetSignOn as u8]);
    dgr.recv_result();

    dgr.send_cmd(&[jtagmk2::Commands::GetSignOn as u8]);
    dgr.recv_result();
    dgr.increase_seqno();
    */

    dgr.sign_on();
    dgr.sign_on();

    /*
    println!(">>> Will set param");
    dgr.send_cmd(&[jtagmk2::Commands::SetParam as u8, 0x03, 0x06]);
    dgr.recv_result();
    dgr.increase_seqno();

    println!(">>> Will get sync");
    dgr.send_cmd(&[jtagmk2::Commands::GetSync as u8]);
    let a = dgr.recv_result();
    dgr.increase_seqno();
    */

    //Set bd rate to 115200
    println!(">>> Will set baud rate");
    dgr.send_cmd(&[jtagmk2::Commands::SetParam as u8, 0x05, 0x07]);
    dgr.recv_result();
    dgr.increase_seqno();
    dgr.port.set_baud_rate(115200);

    /*
    // AT this point we have the device descriptor
    println!("Will set device descriptor");
    dgr.send_cmd(jtagmk2::SET_DEV_DESCRIPTOR);
    let a = dgr.recv_result();
    dgr.increase_seqno();
    */

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
        //        println!(">>> Will enter progmode");
        //        dgr.send_cmd(&[jtagmk2::Commands::EnterProgMode as u8]);
        //        let a = dgr.recv_result();

        let mut ram: Vec<u8> = vec![];
        for mem_addr in 0x3f00..0x3f0f {
            /*
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
            */
            let byte = dgr.read_ram_byte(mem_addr);
            match byte {
                Ok(val) => ram.push(val),
                Err(_) => ram.push(0),
            }
        }

        for chunk in ram.chunks(10) {
            println!("{:02x?}", chunk)
        }

        //        println!(">>> Will run GO");
        //       dgr.send_cmd(&[jtagmk2::Commands::Go as u8]);
        //      let a = dgr.recv_result();

        // Will write a memory byte
        let w_addr = 0x3f00;
        let w_value = 0x88;
        let mut numbytes_buf = [0u8; 2];
        LittleEndian::write_u16(&mut numbytes_buf, 1);

        let mut addr_buf = [0u8; 2];
        LittleEndian::write_u16(&mut addr_buf, w_addr);
        dgr.send_cmd(&[
            jtagmk2::Commands::WriteMemory as u8,
            0x20, // Mem type SRAM
            numbytes_buf[0],
            numbytes_buf[1],
            0,
            0,
            addr_buf[0],
            addr_buf[1],
            0,
            0,
            w_value,
        ]);
        let rcv = dgr.recv_result().unwrap();

        println!("Continue loop ?");
        let mut _buffer = String::new();
        io::stdin().read_line(&mut _buffer);
    }
}
