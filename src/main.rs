use std::{
    io::{self, Write},
    thread,
    time::{self, Duration},
};

use jtagmk2::JtagIceMkii;

mod jtagmk2;

use ramlink::consumer;

struct mk2<'a> {
    dev: JtagIceMkii<'a>,
}

impl<'a> ramlink::consumer::MemoryReader for mk2<'a> {
    fn read_memory(&mut self, address: usize, buffer: &mut [u8]) -> Result<(), String> {
        for i in 0..buffer.len() {
            let byte = self.dev.read_ram_byte((address + i) as u16).unwrap();
            /*
            println!(
                "They ask me to read {} @{:02x?} = {:02x?}",
                i,
                address + i,
                byte
            );
            */
            buffer[i] = byte;
        }
        Ok(())
    }
    fn write_memory(&mut self, address: usize, value: u8) -> Result<(), String> {
        self.dev.write_ram_byte(address as u16, value);
        Ok(())
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

    let mut dgr = jtagmk2::JtagIceMkii::new(port);

    let _ = dgr.sign_on();
    dgr.sign_on().expect("Couldn't sign on");

    //Set bd rate to 115200
    println!(">>> Will set baud rate");
    dgr.send_cmd(&[jtagmk2::Commands::SetParam as u8, 0x05, 0x07]);
    dgr.recv_result().expect("Couldn't set bd rate");
    dgr.increase_seqno();
    dgr.port
        .set_baud_rate(115200)
        .expect("Couldnot set bd rate on serial");

    // XXX Set device descriptor
    println!("Will set device descriptor again ?!");
    dgr.send_cmd(&[jtagmk2::Commands::SetDeviceDescriptor as u8, 0x05, 0x07]);
    dgr.recv_result().expect("Couldn't set device descriptor");
    dgr.increase_seqno();
    dgr.port
        .set_baud_rate(115200)
        .expect("Couldn't set bd rate on serial");

    let mut ram: Vec<u8> = vec![];

    for mem_addr in 0x3f00..=0x3f1a {
        let byte = dgr.read_ram_byte(mem_addr);
        match byte {
            Ok(val) => ram.push(val),
            Err(_) => ram.push(0),
        }
    }

    for chunk in ram.chunks(10) {
        println!("{:02x?}", chunk)
    }

    let mm = mk2 { dev: dgr };

    let mut rb = ramlink::consumer::ProducerDevice::new(Box::new(mm), 0x3f0e).unwrap();

    while true {
        let r = rb.read_bytes();
        if r.len() > 0 {
            println!("I READ {:02x?}", r);
        }
        let ten_millis = time::Duration::from_millis(100);
        thread::sleep(ten_millis)
    }
    //rb.read_byte();
}
