#![feature(str_split_once)]

use byte_unit::Byte;
use clap::{AppSettings, Clap};
use soclose::SoCloseServer;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use std::{fmt, process};

struct Header(Vec<u8>, Vec<u8>);

#[derive(Debug, Clone)]
struct HeaderParseError;

impl Display for HeaderParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "invalid header")
    }
}

impl Error for HeaderParseError {}

impl FromStr for Header {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.split_once(':')
            .map(|(n, v)| Header(n.trim().as_bytes().to_vec(), v.trim().as_bytes().to_vec()))
            .ok_or(HeaderParseError)
    }
}

/// Web server that automatically aborts its dummy responses
/// after a specified amount of data is sent
#[derive(Clap)]
#[clap(
    version = "0.1.0", author = "Niklas Elsbrock <niklas@els-web.de>",
    setting = AppSettings::DeriveDisplayOrder,
    setting = AppSettings::UnifiedHelpMessage,
)]
struct Opts {
    /// Defines how much data the server promises to send.
    #[clap(short, long, value_name = "size", default_value = "100MiB")]
    file_size: Byte,

    /// Defines how much data the server actually sends.
    ///
    /// As soclose uses a block size of 8192 bytes (8KiB), this value gets floored to the next
    /// multiple of 8192 bytes.
    #[clap(short, long, value_name = "size", default_value = "95MiB")]
    send: Byte,

    /// Throttles the data output to a maximum of the given amount of data per second.
    #[clap(short, long, value_name = "size", default_missing_value = "1MiB")]
    throttle: Option<Byte>,

    /// Defines how many seconds the server waits before closing the connection,
    /// after all data is sent.
    #[clap(short, long, value_name = "secs", default_value = "0.0")]
    wait: f64,

    /// Additional headers for each response.
    #[clap(long, value_name = "name:value", min_values = 1)]
    headers: Vec<Header>,

    /// Disables the timeout for TCP read/write operations.
    #[clap(long, conflicts_with = "timeout")]
    no_timeout: bool,

    /// Defines a timeout for TCP read/write operations, in seconds.
    #[clap(long, value_name = "secs", default_value = "10.0")]
    timeout: f64,

    /// Defines which local TCP address the server binds to.
    #[clap(value_name = "bind")]
    bind: SocketAddr,
}

fn main() {
    let opts: Opts = Opts::parse();

    match SoCloseServer::new(
        opts.file_size,
        opts.send,
        opts.throttle,
        Duration::from_secs_f64(opts.wait),
        opts.headers
            .into_iter()
            .map(|Header(n, v)| (n, v))
            .collect(),
        if opts.no_timeout {
            None
        } else {
            Some(Duration::from_secs_f64(opts.timeout))
        },
    )
    .run(opts.bind)
    {
        Ok(_) => unreachable!(),
        Err(err) => {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    }
}
