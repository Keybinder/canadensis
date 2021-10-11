extern crate canadensis;
extern crate canadensis_linux;
extern crate rand;
extern crate socketcan;

use std::convert::TryFrom;
use std::env;
use std::time::Duration;

use socketcan::CANSocket;

use canadensis::can::queue::{ArrayQueue, FrameQueueSource};
use canadensis::can::Mtu;
use canadensis::core::time::{Instant, Microseconds64};
use canadensis::core::transfer::{MessageTransfer, ServiceTransfer};
use canadensis::core::NodeId;
use canadensis::node::{BasicNode, CoreNode};
use canadensis::{Node, ResponseToken, TransferHandler};
use canadensis_data_types::uavcan::node::get_info_1_0::GetInfoResponse;
use canadensis_data_types::uavcan::node::version_1_0::Version;
use canadensis_linux::{LinuxCan, SystemClock};
use std::io::ErrorKind;

/// Runs a basic UAVCAN node that sends Heartbeat messages, responds to node information requests,
/// and sends port list messages
///
/// Usage: `basic_node [SocketCAN interface name] [Node ID]`
///
/// # Testing
///
/// ## Create a virtual CAN device
///
/// ```
/// sudo modprobe vcan
/// sudo ip link add dev vcan0 type vcan
/// sudo ip link set up vcan0
/// ```
///
/// ## Start the node
///
/// ```
/// basic_node vcan0 [node ID]
/// ```
///
/// ## Interact with the node using Yakut
///
/// To subscribe and print out Heartbeat messages:
/// `yakut --transport "CAN(can.media.socketcan.SocketCANMedia('vcan0',8),42)" subscribe uavcan.node.Heartbeat.1.0`
///
/// To send a NodeInfo request:
/// `yakut --transport "CAN(can.media.socketcan.SocketCANMedia('vcan0',8),42)" call [Node ID of basic_node] uavcan.node.GetInfo.1.0 {}`
///
/// In the above two commands, 8 is the MTU of standard CAN and 42 is the node ID of the Yakut node.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let can_interface = args.next().expect("Expected CAN interface name");
    let node_id = NodeId::try_from(
        args.next()
            .expect("Expected node ID")
            .parse::<u8>()
            .expect("Invalid node ID format"),
    )
    .expect("Node ID too large");

    println!(
        "Port list size: {} bytes",
        std::mem::size_of::<canadensis_data_types::uavcan::node::port::list_0_1::List>()
    );

    let can = CANSocket::open(&can_interface).expect("Failed to open CAN interface");
    can.set_read_timeout(Duration::from_millis(500))?;
    can.set_write_timeout(Duration::from_millis(500))?;
    let mut can = LinuxCan::new(can);

    // Set up information about this node
    let node_info = GetInfoResponse {
        protocol_version: Version { major: 1, minor: 0 },
        hardware_version: Version { major: 0, minor: 0 },
        software_version: Version { major: 0, minor: 1 },
        software_vcs_revision_id: 0,
        unique_id: rand::random(),
        name: heapless::Vec::from_slice(b"org.samcrow.basic_node").unwrap(),
        software_image_crc: heapless::Vec::new(),
        certificate_of_authenticity: Default::default(),
    };

    // Create a node with capacity for 8 publishers and 8 requesters
    let core_node: CoreNode<_, _, 8, 8> = CoreNode::new(
        SystemClock::new(),
        node_id,
        Mtu::Can8,
        ArrayQueue::<Microseconds64, 1210>::new(),
    );
    let mut node = BasicNode::new(core_node, node_info).unwrap();

    // Now that the node has subscribed to everything it wants, set up the frame acceptance filters
    let frame_filters = node.frame_filters().unwrap();
    println!("Filters: {:?}", frame_filters);
    can.set_filters(&frame_filters)?;

    println!("Node size: {} bytes", std::mem::size_of_val(&node));

    let start_time = std::time::Instant::now();
    let mut prev_seconds = 0;
    loop {
        match can.receive() {
            Ok(frame) => {
                node.accept_frame(frame, &mut EmptyHandler).unwrap();
            }
            Err(e) => match e.kind() {
                ErrorKind::WouldBlock => {}
                _ => return Err(e.into()),
            },
        };

        let seconds = std::time::Instant::now()
            .duration_since(start_time)
            .as_secs();
        if seconds != prev_seconds {
            prev_seconds = seconds;
            node.run_per_second_tasks().unwrap();
        }

        while let Some(frame_out) = node.frame_queue_mut().pop_frame() {
            can.send(frame_out)?;
        }
    }
}

struct EmptyHandler;

impl<I: Instant> TransferHandler<I> for EmptyHandler {
    fn handle_message<N>(&mut self, _node: &mut N, transfer: &MessageTransfer<Vec<u8>, I>) -> bool
    where
        N: Node<Instant = I>,
    {
        println!("Got message {:?}", transfer);
        false
    }

    fn handle_request<N>(
        &mut self,
        _node: &mut N,
        _token: ResponseToken,
        transfer: &ServiceTransfer<Vec<u8>, I>,
    ) -> bool
    where
        N: Node<Instant = I>,
    {
        println!("Got request {:?}", transfer);
        false
    }

    fn handle_response<N>(&mut self, _node: &mut N, transfer: &ServiceTransfer<Vec<u8>, I>) -> bool
    where
        N: Node<Instant = I>,
    {
        println!("Got response {:?}", transfer);
        false
    }
}
