mod conn;
mod pkt;

use self::conn::Connection;
use self::pkt::Packet;
use crate::glocals::Error;

use failure::err_msg;
use serde::{Deserialize, Serialize};
use std;
use std::collections::hash_map::HashMap;
use std::fmt::Debug;
use std::iter::Iterator;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::time::Duration;

/// Provides an interface to encode and [reliably] send, decode and receive messages to/from any destination.

// - maybe use concurrency for timing rather than polling

////////////
// Socket //
////////////

pub struct Socket<T: Clone + Debug + Eq + PartialEq> {
    socket: UdpSocket,
    connections: HashMap<SocketAddr, Connection<T>>,
}

impl<'a, T: Clone + Debug + Default + Deserialize<'a> + Eq + Serialize + PartialEq> Socket<T> {
    pub fn new(port: u16) -> Result<Socket<T>, Error> {
        Ok(Socket {
            socket: UdpSocket::bind(
                ("127.0.0.1:".to_string() + port.to_string().as_str()).as_str(),
            )?,
            connections: HashMap::new(),
        })
    }

    /// A temporary (TODO) simple (but brute force) solution to the need of occasionally resending
    /// packets which haven't been acknowledged.
    pub fn update(&mut self) -> Result<(), Error> {
        let socket = self.socket.try_clone()?; // because of aliasing
        for (addr, conn) in self.connections.iter_mut() {
            let to_resend = conn.get_resend_queue();
            for pkt in to_resend {
                socket.send_to(&pkt, *addr)?;
            }
        }

        Ok(())
    }

    pub fn max_payload_size() -> u32 {
        Packet::<T>::max_payload_size() // because Packet should probably be private to the net module
    }

    /// Send unreliable message
    pub fn send_to(&mut self, msg: T, dest: SocketAddr) -> Result<(), Error> {
        let buffer = Packet::Unreliable { msg }.encode()?;
        self.socket.send_to(&buffer, dest)?;

        Ok(())
    }

    /// Send reliable message
    pub fn send_reliably_to(&mut self, msg: T, dest: SocketAddr) -> Result<(), Error> {
        // Need to clone it here because of aliasing :/
        // IDEA: Could also let Connection::wrap_message take a clone of the UdpSocket and send the
        // message itself. Connection could even know the UdpSocket and SocketAddr
        let socket = self.socket.try_clone()?;

        let conn = self.get_connection_or_create(dest);
        conn.send_message(msg, &socket)?;
        Ok(())
    }

    fn get_connection_or_create(&mut self, dest: SocketAddr) -> &mut Connection<T> {
        if self.connections.get(&dest).is_none() {
            let conn = Connection::new(dest);
            self.connections.insert(dest, conn);
        }
        self.connections.get_mut(&dest).unwrap()
    }

    // Should wait until the packet is acknowledged..:
    // pub fn send_reliable_to_and_wait(&self, msg: Message, dest: SocketAddr) -> Result<()>

    /// Blocking, with timeout (3 sec)
    pub fn recv(&mut self, buffer: &'a mut [u8]) -> Result<(SocketAddr, T), Error> {
        self.socket.set_nonblocking(false)?;
        self.socket.set_read_timeout(Some(Duration::new(3, 0)))?;
        self._recv(buffer)
    }

    fn _recv(&mut self, buffer: &'a mut [u8]) -> Result<(SocketAddr, T), Error> {
        // Since we may just receive an Ack, we loop until we receive an actual message
        let socket = self.socket.try_clone()?;
        let (_, src) = socket.recv_from(buffer)?;
        let conn = self.get_connection_or_create(src);
        let packet: Packet<T> = Packet::decode(buffer)?;
        // let value = self.get_connection_or_create(src);
        let msg = conn.unwrap_message(packet, &socket)?;
        if let Some(msg) = msg {
            Ok((src, msg))
        } else {
            Err(err_msg("did not recv a message"))
        }
        // let conn = self.get_connection_or_create(src);
        // let msg = conn.unwrap_message(packet, &socket)?;
        // if let Some(msg) = msg {
        //     break (src, msg);
        // }
    }
}

pub fn to_socket_addr(addr: &str) -> Result<SocketAddr, Error> {
    // Assume IPv4. Try to parse.
    let parts: Vec<&str> = addr.split(':').collect();
    if parts.len() != 2 {
        panic!("IP address must be on the form X.X.X.X:port");
    }

    let addr: Vec<u8> = parts[0]
        .split('.')
        .map(|x| x.parse::<u8>().unwrap())
        .collect();
    if addr.len() != 4 {
        panic!("IP address must be on the form X.X.X.X:port");
    }

    let port = parts[1].parse::<u16>().unwrap();

    Ok(SocketAddr::V4(SocketAddrV4::new(
        Ipv4Addr::new(addr[0], addr[1], addr[2], addr[3]),
        port,
    )))
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use super::*;
    use test::{black_box, Bencher};

    static CLIENT_PORT: u16 = 12347;
    static SERVER_PORT: u16 = 34254;

    #[test]
    fn confirm_message_arrives() {
        let mut client: Socket<bool> = Socket::new(CLIENT_PORT).unwrap();
        let mut server: Socket<bool> = Socket::new(SERVER_PORT).unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), SERVER_PORT);
        client.send_to(false, destination).unwrap();
        let mut buffer = [0u8; 1000];
        server.recv(&mut buffer).unwrap();
    }

    #[test]
    fn confirm_reliable_message_arrives() {
        let mut client: Socket<bool> = Socket::new(CLIENT_PORT).unwrap();
        let mut server: Socket<bool> = Socket::new(SERVER_PORT).unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), SERVER_PORT);

        client.send_reliably_to(true, destination).unwrap();
        let mut buffer = [0u8; 3000];
        assert_eq![true, server.recv(&mut buffer).unwrap().1];
        assert![client.get_connection_or_create(destination).send_window[0].is_some()];
        assert![client.recv(&mut buffer).is_err()];
        assert![client.get_connection_or_create(destination).send_window[0].is_none()];
    }

    #[test]
    fn confirm_u8_size_of_packet() {
        assert_eq![8, Packet::Unreliable { msg: 128 }.encode().unwrap().len()];
    }

    // ---

    #[bench]
    fn time_per_byte(b: &mut Bencher) {
        let mut client: Socket<u8> = Socket::new(CLIENT_PORT).unwrap();
        let mut server: Socket<u8> = Socket::new(SERVER_PORT).unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), SERVER_PORT);
        b.iter(|| {
            client.send_to(black_box(128u8), black_box(destination)).unwrap();
            let mut buffer = [0u8; 8];
            server.recv(black_box(&mut buffer)).unwrap();
        });
    }

    #[bench]
    fn allocating_1000_bytes(b: &mut Bencher) {
        b.iter(|| {
            black_box(vec![128; 1000])
        });
    }

    #[bench]
    fn time_per_kilobyte(b: &mut Bencher) {
        let mut client: Socket<Vec<u8>> = Socket::new(CLIENT_PORT).unwrap();
        let mut server: Socket<Vec<u8>> = Socket::new(SERVER_PORT).unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), SERVER_PORT);
        b.iter(|| {
            client.send_to(black_box(vec![128; 1000]), black_box(destination)).unwrap();
            let mut buffer = [0u8; 2000];
            server.recv(black_box(&mut buffer)).unwrap();
        });
    }

    #[bench]
    fn time_per_10_kb(b: &mut Bencher) {
        let mut client: Socket<Vec<u8>> = Socket::new(CLIENT_PORT).unwrap();
        let mut server: Socket<Vec<u8>> = Socket::new(SERVER_PORT).unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), SERVER_PORT);
        b.iter(|| {
            client.send_to(black_box(vec![128; 10_000]), black_box(destination)).unwrap();
            let mut buffer = [0u8; 20_000];
            server.recv(black_box(&mut buffer)).unwrap();
        });
    }
}
