//!
//! An anonymous node that monitors for uavcan.node.Diagnostic.1.0 messages and prints them out
//!
//! Usage: diagnostic_console CAN-interface-name
//!

extern crate canadensis;
extern crate canadensis_data_types;
extern crate socketcan;

use std::convert::TryFrom;
use std::env;
use std::error::Error;
use std::process;
use std::time::Instant;

use canadensis::core::time::{Clock, MicrosecondDuration64, Microseconds64};
use canadensis::core::transport::Receiver;
use canadensis::encoding::{DataType, Deserialize, ReadCursor};
use canadensis_can::{CanId, CanReceiver, Frame, Mtu};
use canadensis_data_types::uavcan::diagnostic::record_1_1::{self, Record};
use canadensis_data_types::uavcan::diagnostic::severity_1_0::Severity;

fn main() -> Result<(), Box<dyn Error>> {
    let interface = env::args().skip(1).next().unwrap_or_else(|| {
        eprintln!("Expected a SocketCAN interface name");
        process::exit(-1);
    });
    let can = socketcan::CANSocket::open(&interface)?;

    let mut clock = SystemClock::new();
    let mut receiver = CanReceiver::new_anonymous(Mtu::Can8);
    receiver
        .subscribe_message(
            record_1_1::SUBJECT,
            Record::EXTENT_BYTES.unwrap() as usize,
            MicrosecondDuration64::new(1_000_000),
        )
        .unwrap();

    loop {
        let frame = can.read_frame()?;
        // Convert from SocketCAN to Canadensis
        let frame = Frame::new(
            clock.now(),
            CanId::try_from(frame.id()).unwrap(),
            frame.data(),
        );
        if let Some(transfer) = receiver.accept(frame).unwrap() {
            match Record::deserialize(&mut ReadCursor::new(&transfer.payload)) {
                Ok(log_record) => {
                    let node_text = transfer
                        .header
                        .source()
                        .map(|node| node.to_string())
                        .unwrap_or_else(|| "?".to_owned());
                    let level_text = match log_record.severity.value {
                        Severity::TRACE => 'T',
                        Severity::DEBUG => 'D',
                        Severity::INFO => 'I',
                        Severity::NOTICE => 'N',
                        Severity::WARNING => 'W',
                        Severity::ERROR => 'E',
                        Severity::CRITICAL => 'C',
                        Severity::ALERT => 'A',
                        _ => '?',
                    };
                    let text = String::from_utf8_lossy(&log_record.text);

                    println!(
                        "[{node}][{level}] {text}",
                        node = node_text,
                        level = level_text,
                        text = text
                    );
                }
                Err(e) => eprintln!("Couldn't deserialize log record: {:?}", e),
            }
        }
    }
}

#[derive(Debug, Clone)]
struct SystemClock {
    start_time: Instant,
}

impl SystemClock {
    pub fn new() -> Self {
        SystemClock {
            start_time: Instant::now(),
        }
    }
}

impl Clock for SystemClock {
    type Instant = Microseconds64;

    fn now(&mut self) -> Self::Instant {
        let since_start = Instant::now().duration_since(self.start_time);
        let microseconds = since_start.as_micros();
        Microseconds64::new(microseconds as u64)
    }
}
