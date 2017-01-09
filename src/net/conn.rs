use std::hash::{Hash, Hasher};
use std::collections::VecDeque;
use net::msg::Message;
use net::pkt::{Packet, PacketKind};
use std::net::SocketAddr;
use err::Result;

#[derive(Clone)]
struct SendWindow {
    /// Sequence number of first packet in the queue
    pub start_seq: u32,
    /// The time that the first packet in the queue was sent
    pub start_time: u64,
    /// Packets having been sent but not yet acknowledged
    pub packets: VecDeque<Vec<u8>>,
}

impl SendWindow {
    pub fn new() -> SendWindow {
        SendWindow {
            start_seq: 0,
            start_time: 0,
            packets: VecDeque::new(),
        }
    }
}


#[derive(Clone)]
pub struct Connection {
    pub received: u32,
    /// The ack number of the other part
    pub acked: u32,
    /// The sequence number of the next sent packet
    pub seq: u32,

    pub send_window: SendWindow,
}

impl<'a> Connection {
    pub fn new() -> Connection {
        Connection {
            received: 0,
            acked: 0,
            seq: 0,
            send_window: SendWindow::new(),
        }
    }

    pub fn acknowledge(ack: u32) {
        // TODO Remove some packets from the queue
    }


    /// Wraps in a packet, encodes, and adds the packet to the send window queue. Returns the data
    /// enqueued.
    pub fn wrap_message(&'a mut self, msg: Message) -> &'a Vec<u8> {
        let packet = Packet::new(PacketKind::Reliable {ack: self.received+1, seq: self.seq}, msg);
        let encoded = packet.encode();
        self.seq += 1;

        self.send_window.packets.push_back(encoded);
        &self.send_window.packets.back().unwrap()
    }

    /// Unwraps packet, and if it is reliable, update `self.ack` to acknowledge it later.
    pub fn unwrap_message(&mut self, data: &[u8]) -> Result<Message> {
        let p = Packet::decode(data)?;
        p.check_protocol_nr()?;
        match p.kind {
            PacketKind::Unreliable {ack} => {},
            PacketKind::Reliable {ack, seq} => {
                self.received = seq;
                self.acked = ack;
            },
        };
        Ok(p.msg)
    }

    /// Wraps and encodes the message but doesn't change its own state.
    pub fn wrap_unreliable_message(&self, msg: Message) -> Vec<u8> {
        let packet = Packet::new(PacketKind::Unreliable {ack: self.received+1}, msg);
        packet.encode()
    }
    
}
