mod utils;

use std::io::{ Read, Write };
use std::net::TcpListener;
use utils::*;

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("0.0.0.0:51413")?;

    // accept incoming connections and process them serially
    Ok(for stream in listener.incoming() {
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
                        "max": 20,
                        "online": 1,
                        "sample": [
                            {
                                "name": "ReeZey",
                                "id": "2a350988-50ac-41df-b274-1b5eb6e633c1"
                            }
                        ]
                    },
                    "description": {
                        "text": "Hello world"
                    },
                    "enforcesSecureChat": false
                }
                "#;

                match packedid {
                    0 => {
                        let _version = read_var_int(&mut stream).unwrap();

                        let address_len = read_var_int(&mut stream).unwrap() as usize;
                        let _address = read_string(&mut stream, address_len);
                        let _port = u16::from_be_bytes(read_bytes(&mut stream, 2).try_into().unwrap());

                        //println!("{address}:{port}");

                        let nextstate = read_var_int(&mut stream).unwrap();

                        match nextstate {
                            1 => {
                                let mut buffer: Vec<u8> = vec![];
                                write_string(&mut buffer, status.to_string());
                                flush(&mut stream, &mut buffer, 0);
                                buffer.clear();

                                let mut test: [u8; 8] = Default::default();
                                stream.read(&mut test).unwrap();
                                buffer.write(&test).unwrap();
                                flush(&mut stream, &mut buffer, 1);
                            },
                            2 => {
                                let _connectionid = read_var_int(&mut stream).unwrap();
                                let _identifier = read_var_int(&mut stream).unwrap();
                                
                                let user_len = read_var_int(&mut stream).unwrap();
                                let _username = read_string(&mut stream, user_len as usize);
                                //println!("{connectionid} {identifier} {username}");

                                let mut buffer: Vec<u8> = vec![];
                                write_string(&mut buffer, "fakjo".to_string());
                                flush(&mut stream, &mut buffer, 0);
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
    })

}