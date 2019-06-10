use super::pkt::Packet;
use failure::{bail, Error};
use serde::{Deserialize, Serialize};
use std::{
    self,
    collections::VecDeque,
    fmt::Debug,
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

pub struct SentPacket<T: Clone + Debug + PartialEq> {
    pub time: Instant,
    pub seq: u32,
    pub packet: Packet<T>,
}

impl<'a, T: Clone + Debug + PartialEq> Debug for SentPacket<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "SentPacket; time = {:?}, seq = {}", self.time, self.seq)
    }
}

#[derive(Debug)]
pub struct Connection<T: Clone + Debug + PartialEq> {
    /// The sequence number of the next sent packet
    pub seq: u32,
    /// The first entry should always be Some.
    /// Some means that it's not yet acknowledged
    pub send_window: VecDeque<Option<SentPacket<T>>>,
}

impl<T: Clone + Debug + PartialEq> Connection<T> {
    pub fn new() -> Connection<T> {
        Connection {
            seq: 0,
            send_window: VecDeque::new(),
        }
    }

    /// Returns Vec of encoded packets ready to be sent again
    pub fn resend_fast(
        &mut self,
        time: Instant,
        socket: &UdpSocket,
        dest: SocketAddr,
    ) -> Result<bool, Error>
    where
        T: Serialize,
    {
        let now = time;
        self.update_send_window();
        let mut seen = false;
        for sent_packet in self.send_window.iter_mut() {
            if let Some(ref mut sent_packet) = *sent_packet {
                if now - sent_packet.time >= Duration::new(1, 0) {
                    sent_packet.time = now;
                    socket.send_to(&sent_packet.packet.encode().unwrap()[..], dest)?;
                    seen = true;
                } else {
                    break;
                }
            }
        }
        Ok(seen)
    }

    pub fn acknowledge(&mut self, acked: u32) -> Result<(), Error> {
        self.update_send_window();
        // Get the seq number of the first element
        let first_seq = match self.send_window.front() {
            None => {
                bail!["not good"];
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
        while let Some(&None) = self.send_window.front() {
            self.send_window.pop_front();
        }
    }

    /// Wraps in a packet, encodes, and adds the packet to the send window queue. Returns the data
    /// enqueued.
    pub fn send_message<'b>(
        &'b mut self,
        msg: T,
        time: Instant,
        socket: &UdpSocket,
        dest: SocketAddr,
    ) -> Result<u32, Error>
    where
        T: Serialize,
    {
        let packet = Packet::Reliable { seq: self.seq, msg };
        let pkt_bytes = packet.encode()?;
        // debug!("Send"; "seq" => self.seq, "ack" => self.received+1);
        self.send_window.push_back(Some(SentPacket {
            time,
            seq: self.seq,
            packet: packet.clone(),
        }));

        self.seq += 1;
        socket.send_to(&pkt_bytes, dest)?;
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
        dest: SocketAddr,
    ) -> Result<Option<T>, Error>
    where
        T: Serialize,
    {
        match packet {
            Packet::Unreliable { msg } => Ok(Some(msg)),
            Packet::Reliable { seq, msg } => {
                // ack_reply = Some(Packet::Ack {ack: seq});
                let packet: Packet<T> = Packet::Ack { ack: seq };
                socket.send_to(&packet.encode()?, dest)?;
                Ok(Some(msg))
            }
            Packet::Ack { ack } => {
                self.acknowledge(ack)?;
                Ok(None)
            }
        }
    }
}
