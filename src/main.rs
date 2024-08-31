use std::{thread, time};
use std::fs::File;
use std::io::{self, Read, Write};
use std::io::ErrorKind::NotFound;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use serialport::ErrorKind::{Io, NoDevice};
use serialport::SerialPort;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The device path to a serial port
    port: String,

    /// The baud rate to connect at
    #[arg(short, long, default_value_t = 115_200)]
    baud: u32,

    /// read form this file
    #[clap(short, long)]
    input_file: Option<PathBuf>,

    /// Wait up to 10 seconds for the serial port to appear
    #[arg(short, long, default_value_t = false)]
    wait: bool,
}

const WAIT_MILLISECONDS: u64 = 10_000;
const WAIT_INTERVAL_MILLISECONDS: u64 = 100;

fn main() {
    let cli = Cli::parse();

    let port = open_port_retrying(&cli);

    match port {
        Ok(mut port) => {
            if let Some(path) = cli.input_file {
                let mut file = File::open(&path).unwrap();
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer).unwrap();
                port.write_all(&buffer).unwrap();
                thread::sleep(Duration::from_millis(500));
                eprintln!("Wrote {} to {} at {} baud", path.as_os_str().to_str().unwrap(), &cli.port, &cli.baud);
            } else {

                // Clone the port
                let mut clone = port.try_clone().expect("Failed to clone");
                // Send out 4 bytes every second
                thread::spawn(move || loop {
                    for i in io::stdin().bytes() {
                        clone
                            .write_all(&[i.unwrap()])
                            .expect("Failed to write to serial port");
                    }
                });

                let mut serial_buf: Vec<u8> = vec![0; 1000];
                eprintln!("Opened {} at {} baud", &cli.port, &cli.baud);
                loop {
                    match port.read(serial_buf.as_mut_slice()) {
                        Ok(t) => io::stdout().write_all(&serial_buf[..t]).unwrap(),
                        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                        Err(e) => {
                            eprintln!("{:?}", e);
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to open \"{}\". Error: {e}", cli.port);
            std::process::exit(1);
        }
    }
}

fn open_port_retrying(cli: &Cli) -> Result<Box<dyn SerialPort>, String> {
    return if cli.wait {
        let retries = WAIT_MILLISECONDS / WAIT_INTERVAL_MILLISECONDS;
        let delay = time::Duration::from_millis(WAIT_INTERVAL_MILLISECONDS);

        for _ in 1..retries {
            match open_port(cli) {
                Ok(p) => return Ok(p),
                Err(e) => {
                    match e.kind {
                        NoDevice => {}
                        Io(NotFound) => {}
                        _ => return Err(e.to_string())
                    }
                    thread::sleep(delay);
                }
            }
        }
        Err(format!("Failed to open device after {} seconds", WAIT_MILLISECONDS / 1_000))
    } else {
        match open_port(cli) {
            Ok(p) => Ok(p),
            Err(e) => Err(e.to_string()),
        }
    };
}

fn open_port(cli: &Cli) -> serialport::Result<Box<dyn SerialPort>> {
    return serialport::new(cli.port.clone(), cli.baud)
        .timeout(Duration::from_millis(10))
        .open();
}
