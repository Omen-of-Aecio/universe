use super::pkt::Packet;
use failure::Error;
use serde::Serialize;
use std::{
    self,
    collections::HashMap,
    fmt::Debug,
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

// ---

pub struct SentPacket<T: Clone + Debug + PartialEq> {
    /// The time the packet was sent, used for re-sending after a given time
    pub time: Instant,
    /// The contents of the packet
    pub packet: Packet<T>,
}

impl<'a, T: Clone + Debug + PartialEq> Debug for SentPacket<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "SentPacket; time = {:?}", self.time)
    }
}

#[derive(Debug, Default)]
pub struct Connection<T: Clone + Debug + PartialEq> {
    /// The sequence number of the next sent packet, incremented after sending a packet
    pub seq: u32,
    /// Send window for packets
    ///
    /// When receiving an acknowledgement we remove the packet from the map.
    pub send_window: HashMap<u32, SentPacket<T>>,
}

impl<T: Clone + Debug + PartialEq> Connection<T> {
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
        let mut seen = false;
        for (_, sent_packet) in self.send_window.iter_mut() {
            if now - sent_packet.time >= Duration::new(1, 0) {
                sent_packet.time = now;
                socket.send_to(&sent_packet.packet.encode().unwrap()[..], dest)?;
                seen = true;
            } else {
                break;
            }
        }
        Ok(seen)
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
        self.send_window.insert(
            self.seq,
            SentPacket {
                time,
                packet: packet.clone(),
            },
        );

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
                self.acknowledge(ack);
                Ok(None)
            }
        }
    }

    fn acknowledge(&mut self, acked: u32) {
        self.send_window.remove(&acked);
    }
}
