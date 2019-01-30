use super::pkt::Packet;
use crate::glocals::Error;
use serde::{Deserialize, Serialize};
use std::{
    self,
    collections::VecDeque,
    fmt::Debug,
    net::{SocketAddr, UdpSocket},
};
use time::precise_time_ns;

pub struct SentPacket<T: Clone + Debug + Eq + PartialEq> {
    pub time: u64,
    pub seq: u32,
    pub packet: Packet<T>,
}

impl<'a, T: Clone + Debug + Deserialize<'a> + Eq + Serialize + PartialEq> Debug for SentPacket<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "SentPacket; time = {}, seq = {}", self.time, self.seq)
    }
}

pub struct Connection<T: Clone + Debug + Eq + PartialEq> {
    /// The sequence number of the next sent packet
    pub seq: u32,
    /// The first entry should always be Some.
    /// Some means that it's not yet acknowledged
    pub send_window: VecDeque<Option<SentPacket<T>>>,

    dest: SocketAddr,
}
const RESEND_INTERVAL_MS: u64 = 1000;

impl<'a, T: Clone + Debug + Deserialize<'a> + Eq + Serialize + PartialEq> Connection<T> {
    pub fn new(dest: SocketAddr) -> Connection<T> {
        Connection {
            seq: 0,
            send_window: VecDeque::new(),
            dest,
        }
    }

    /// Returns Vec of encoded packets ready to be sent again
    pub fn get_resend_queue(&mut self) -> Vec<Vec<u8>> {
        let now = precise_time_ns();
        self.update_send_window();
        let mut result = Vec::new();
        for sent_packet in self.send_window.iter_mut() {
            if let Some(ref mut sent_packet) = *sent_packet {
                if now > sent_packet.time + RESEND_INTERVAL_MS * 1_000_000 {
                    sent_packet.time = now;
                    result.push(sent_packet.packet.encode().unwrap());
                }
            }
        }
        result
    }

    pub fn acknowledge(&mut self, acked: u32) -> Result<(), Error> {
        self.update_send_window();
        // Get the seq number of the first element
        let first_seq = match self.send_window.front() {
            None => {
                panic!("Send window empty, but ack received.");
            }
            Some(first) => match *first {
                Some(ref sent_packet) => sent_packet.seq,
                None => panic!("The first SentPacket is None."),
            },
        };

        let index = (acked - first_seq) as usize;

        match self.send_window.get_mut(index) {
            Some(sent_packet) => {
                *sent_packet = None;
            }
            None => panic!("Index out of bounds: {}", index),
        };

        Ok(())
    }

    /// Removes all None's that appear at the front of the send window queue
    fn update_send_window(&mut self) {
        loop {
            let remove = match self.send_window.front() {
                Some(&None) => true,
                _ => false,
            };
            if remove {
                self.send_window.pop_front();
            } else {
                break;
            }
        }
    }

    /// Wraps in a packet, encodes, and adds the packet to the send window queue. Returns the data
    /// enqueued.
    pub fn send_message<'b>(&'b mut self, msg: T, socket: &UdpSocket) -> Result<u32, Error> {
        let packet = Packet::Reliable { seq: self.seq, msg };
        // debug!("Send"; "seq" => self.seq, "ack" => self.received+1);
        self.send_window.push_back(Some(SentPacket {
            time: precise_time_ns(),
            seq: self.seq,
            packet: packet.clone(),
        }));

        self.seq += 1;
        socket.send_to(&packet.encode().unwrap(), self.dest)?;
        Ok(self.seq - 1)
    }

    /// Unwraps message from packet. If reliable, it will return Some(Packet) which should be sent
    /// as an acknowledgement. Needs `UdpSocket` for sending pack an eventual Ack
    // Ideally, I would like to take a &[u8] here but it creates aliasing conflicts, as Socket will
    // have to send a slice of its own buffer.
    pub fn unwrap_message(
        &mut self,
        packet: Packet<T>,
        socket: &UdpSocket,
    ) -> Result<Option<T>, Error> {
        let mut received_msg = None;
        match packet {
            Packet::Unreliable { msg } => {
                received_msg = Some(msg);
            }
            Packet::Reliable { seq, msg } => {
                received_msg = Some(msg);
                // ack_reply = Some(Packet::Ack {ack: seq});
                let packet: Packet<T> = Packet::Ack { ack: seq };
                socket.send_to(&packet.encode()?, self.dest)?;
            }
            Packet::Ack { ack } => {
                self.acknowledge(ack)?;
            }
        };
        Ok(received_msg)
    }
}
