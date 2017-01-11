use std::collections::VecDeque;
use net::msg::Message;
use net::pkt::{Packet, PacketKind};
use err::Result;
use time::precise_time_ns;
use std;

#[derive(Clone)]
pub struct SentPacket {
    pub time: u64,
    pub seq: u32,
    pub packet: Packet,
}
#[derive(Clone)]
pub struct SendWindow {
    /// Packets having been sent but not yet acknowledged
    pub packets: VecDeque<SentPacket>,
    // TODO when imposing limit - additional queue for NOT SENT
}

impl SendWindow {
    pub fn new() -> SendWindow {
        SendWindow {
            packets: VecDeque::new(),
        }
    }
}


#[derive(Clone)]
pub struct Connection {
    pub received: u32,
    /// The ack number of the other part
    // TODO I think it is in fact unnecessary to store this one..
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

    /// The time (ns) at which the first packet in the send window queue was sent, if any.
    pub fn first_unacked_packet(&self) -> Option<u64> {
        self.send_window.packets.front().map(|x| x.time)
    }

    /// Consume the queue (send_window) of packages
    pub fn consume_queue(&mut self) -> VecDeque<SentPacket> {
        {
            let s = self.send_window.packets.front();
            if let Some(p) = s { self.seq = p.seq; }
        }

        std::mem::replace(&mut self.send_window.packets, VecDeque::new())
    }

    pub fn acknowledge(&mut self, ack: u32) {
        self.acked = ack;
        loop {
            let acknowledged = {
                self.send_window.packets.front().map(|x| ack > x.seq).unwrap_or(false)
            };
            if acknowledged {
                self.send_window.packets.pop_front();
            } else {
                break;
            }

        }
    }


    /// Wraps in a packet, encodes, and adds the packet to the send window queue. Returns the data
    /// enqueued.
    pub fn wrap_message(&mut self, msg: Message) -> Vec<u8> {
        let packet = Packet::new(PacketKind::Reliable {ack: self.received+1, seq: self.seq}, msg);

        self.send_window.packets.push_back(
            SentPacket {
                time: precise_time_ns(),
                seq: self.seq,
                packet: packet.clone(),
            });

        self.seq += 1;
        packet.encode()
    }

    /// Unwraps packet, and if it is reliable, update `self.ack` to acknowledge it later.
    // Ideally, I would like to take a &[u8] here but it creates aliasing conflicts, as Socket will
    // have to send a slice of its own buffer.
    pub fn unwrap_message(&mut self, packet: Packet) -> Result<Message> {
        packet.check_protocol_nr()?;
        match packet.kind {
            PacketKind::Unreliable {ack} => {},
            PacketKind::Reliable {ack, seq} => {
                self.received = seq;
                self.acknowledge(ack);
            },
        };
        Ok(packet.msg)
    }

    /// Wraps and encodes the message but doesn't change its own state.
    pub fn wrap_unreliable_message(&self, msg: Message) -> Vec<u8> {
        let packet = Packet::new(PacketKind::Unreliable {ack: self.received+1}, msg);
        packet.encode()
    }
    
}
