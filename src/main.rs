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
        for i in 0..address {
            let byte = self.dev.read_ram_byte((address + i) as u16).unwrap();
            buffer[i] = byte;
        }
        buffer[0] = 1;
        Ok(())
    }
    fn write_memory(&self, address: usize, value: u8) -> Result<(), String> {
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

    //*/
    //for mem_addr in 0x3f00..0x3fff {
    //for mem_addr in 0x8000..0x8010 {
    let mut rcvd: Vec<u8> = vec![];
    let mut times = 0;
    loop {
        io::stdout().flush().unwrap();
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

        //for mem_addr in 0x3f00..0x3f0f {
        let prod_a: u16 = 0x3f11;
        let cons_a: u16 = 0x3f12;
        let buff_a: u16 = 0x3f13;

        let prod_v = dgr.read_ram_byte(prod_a).unwrap();
        let cons_v = dgr.read_ram_byte(cons_a).unwrap();

        //println!("P: {:02x?} - C: {:02x?}", prod_v, cons_v);

        if prod_v == cons_v {
            // buffer is empty
            print!(".");
            let ten_millis = time::Duration::from_millis(100);
            thread::sleep(ten_millis);
        } else {
            //if (prod_v != (cons_v + 1) % 5) {
            let buff_a = buff_a + (cons_v as u16);
            let buff_v = dgr.read_ram_byte(buff_a);
            //println!("\n");
            let new_consv = (cons_v + 1) % 5;
            /*
            println!(
                "P: {:02x?} - C: {:02x?} - NC: {:02x?}",
                prod_v, cons_v, new_consv,
            );
            println!("@{:02x?} = {:02x?}", buff_a, buff_v);
            */
            /*
            match buff_v {
                Ok(msg) => rcvd.append(msg),
                Err(_) => (),
            }
            rcvd.append(buff_v.into_ok());
            */
            println!("RCVD: {:02x?}", buff_v);
            dgr.write_ram_byte(cons_a, new_consv);
        }

        /*

        /*
        dgr.write_ram_byte(0x3f00, 0x43)
            .expect("Couldn't write ram");
            */

        println!("Continue loop ?");
        let mut _buffer = String::new();
        io::stdin().read_line(&mut _buffer);
        */
    }
}
