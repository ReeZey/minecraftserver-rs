mod utils;

use std::net::TcpListener;
use utils::{read_var_int, read_string, read_bytes, write_var_int, write_string};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:25565")?;

    // accept incoming connections and process them serially
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                let _length = read_var_int(&mut stream).unwrap();
                let packedid = read_var_int(&mut stream).unwrap();

                //println!("{} {}", length, packedid);


                let status = r#"
                {
                    "version": {
                        "name": "1.19.3",
                        "protocol": 761
                    },
                    "players": {
                        "max": 100,
                        "online": 5,
                        "sample": [
                            {
                                "name": "thinkofdeath",
                                "id": "4566e69f-c907-48ee-8d71-d7ba5aa00d20"
                            }
                        ]
                    },
                    "description": {
                        "text": "Hello world"
                    },
                    "enforcesSecureChat": true
                }
                "#;

                match packedid {
                    0 => {
                        let _version = read_var_int(&mut stream).unwrap();

                        let address_len = read_var_int(&mut stream).unwrap() as usize;
                        let address = read_string(&mut stream, address_len);
                        let port = u16::from_be_bytes(read_bytes(&mut stream, 2).try_into().unwrap());

                        println!("{}:{}", address, port);

                        let nextstate = read_var_int(&mut stream).unwrap();

                        match nextstate {
                            1 => {
                                write_var_int(&mut stream, status.len() as i32);
                                write_string(&mut stream, status.to_string());
                            },
                            2 => {

                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
            }
        }
    }

    Ok(())
}