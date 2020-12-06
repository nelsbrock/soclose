#![feature(duration_zero)]
#![feature(never_type)]

use byte_unit::Byte;
use std::collections::HashMap;
use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;
use std::{hint, io, thread};

const BLOCK_SIZE: usize = 8192;
const BLOCK: [u8; BLOCK_SIZE] = [0; BLOCK_SIZE];

#[derive(Clone)]
pub struct SoCloseServer {
    send_block_count: u64,
    sleep_between: Duration,
    sleep_after: Duration,
    response_head: Vec<u8>,
    timeout: Option<Duration>,
}

impl SoCloseServer {
    pub fn new(
        file_size: Byte,
        send: Byte,
        throttle: Option<Byte>,
        sleep_after: Duration,
        mut headers: HashMap<Vec<u8>, Vec<u8>>,
        timeout: Option<Duration>,
    ) -> SoCloseServer {
        headers.insert(
            b"Content-Length".to_vec(),
            file_size.get_bytes().to_string().into_bytes(),
        );

        SoCloseServer {
            send_block_count: send.get_bytes() / BLOCK_SIZE as u64,
            sleep_between: match throttle {
                Some(throttle) => Duration::from_nanos(
                    (1_000_000_000f64 * (BLOCK_SIZE as f64 / throttle.get_bytes() as f64)) as u64,
                ),
                None => Duration::ZERO,
            },
            sleep_after,
            response_head: {
                let mut head = b"HTTP/1.0 200 OK\r\n".to_vec();
                headers.into_iter().for_each(|(mut n, mut v)| {
                    head.append(&mut n);
                    head.append(&mut b": ".to_vec());
                    head.append(&mut v);
                    head.append(&mut b"\r\n".to_vec());
                });
                head.append(&mut b"\r\n".to_vec());
                head
            },
            timeout,
        }
    }

    pub fn run(self, bind: SocketAddr) -> io::Result<!> {
        let listener = TcpListener::bind(bind)?;

        for (conn_id, stream) in listener.incoming().enumerate() {
            let stream = match stream {
                Ok(stream) => stream,
                Err(_) => continue,
            };

            let c_self = self.clone();

            thread::spawn(move || {
                let peer_addr = match stream.peer_addr() {
                    Ok(peer_addr) => peer_addr,
                    Err(_) => return,
                };
                println!(
                    "[{}] Handling connection from {} \u{2026}",
                    conn_id, peer_addr
                );

                match c_self.handle_connection(stream) {
                    Ok(_) => {
                        println!("[{}] All data sent.", conn_id);
                    }
                    Err(err) => println!("[{}] Connection closed due to error: {}", conn_id, err),
                }
            });
        }

        // SAFETY: The iterator returned by `TcpListener::incoming` never returns `None` and the
        // above for-loop does not contain a `break` statement, thus this point is unreachable.
        unsafe { hint::unreachable_unchecked() }
    }

    fn handle_connection(&self, mut stream: TcpStream) -> io::Result<()> {
        stream.set_read_timeout(self.timeout)?;
        stream.set_write_timeout(self.timeout)?;

        // We only get the first 4 bytes of the request here,
        // in order to check if they equal b"GET ".
        let mut rx_buffer = [0; 4];
        stream.read_exact(&mut rx_buffer)?;
        if &rx_buffer != b"GET " {
            stream.write_all(b"HTTP/1.1 405 Method Not Allowed\r\n\r\n")?;
            return Err(io::Error::new(ErrorKind::InvalidData, "not a GET request"));
        }

        stream.write_all(&*self.response_head)?;
        for _ in 0..self.send_block_count {
            stream.write_all(&BLOCK)?;
            thread::sleep(self.sleep_between);
        }

        thread::sleep(self.sleep_after);

        Ok(())
    }
}
