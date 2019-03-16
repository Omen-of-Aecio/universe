#![feature(test)]
extern crate test;

mod conn;
mod pkt;

use self::conn::Connection;
use self::pkt::Packet;
use failure::Error;
use failure::{bail, ensure};
use serde::{Deserialize, Serialize};
use std;
use std::collections::hash_map::HashMap;
use std::fmt::Debug;
use std::iter::Iterator;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};

/// Provides an interface to encode and relatively reliably send, decode and receive messages to/from any destination.

// - maybe use concurrency for timing rather than polling

////////////
// Socket //
////////////

pub struct Socket<T: Clone + Debug + PartialEq> {
    socket: UdpSocket,
    connections: HashMap<SocketAddr, Connection<T>>,
}

impl<T: Clone + Debug + Default + PartialEq> Socket<T> {
    pub fn new(port: u16) -> Result<Socket<T>, Error> {
        Ok(Socket {
            socket: UdpSocket::bind(
                ("127.0.0.1:".to_string() + port.to_string().as_str()).as_str(),
            )?,
            connections: HashMap::new(),
        })
    }

    pub fn new_with_random_port() -> Result<(Socket<T>, u16), Error> {
        for port in 10000..=u16::max_value() {
            if let Ok(sock) = Self::new(port) {
                return Ok((sock, port));
            }
        }
        bail!["Unable to find a port"]
    }

    /// A temporary (TODO) simple (but brute force) solution to the need of occasionally resending
    /// packets which haven't been acknowledged.
    pub fn update(&mut self) -> Result<(), Error>
    where
        T: Serialize,
    {
        for (addr, conn) in self.connections.iter_mut() {
            let to_resend = conn.get_resend_queue();
            for pkt in to_resend {
                self.socket.send_to(&pkt, *addr)?;
            }
        }
        Ok(())
    }

    /// Send unreliable message
    pub fn send_to(&mut self, msg: T, dest: SocketAddr) -> Result<(), Error>
    where
        T: Serialize,
    {
        let buffer = Packet::Unreliable { msg }.encode()?;
        let size = self.socket.send_to(&buffer, dest)?;
        ensure![
            size == buffer.len(),
            "Entire buffer not sent in a single packet"
        ];
        Ok(())
    }

    fn get_connection_or_create(&mut self, dest: SocketAddr) -> &mut Connection<T> {
        if self.connections.get(&dest).is_none() {
            let conn = Connection::new(dest);
            self.connections.insert(dest, conn);
        }
        self.connections.get_mut(&dest).unwrap()
    }

    /// Send reliable message
    pub fn send_reliably_to(&mut self, msg: T, dest: SocketAddr) -> Result<(), Error>
    where
        T: Serialize,
    {
        if self.connections.get(&dest).is_none() {
            let conn = Connection::new(dest);
            self.connections.insert(dest, conn);
        }
        let conn = self.connections.get_mut(&dest).unwrap();

        conn.send_message(msg, &self.socket)?;
        Ok(())
    }

    // Should wait until the packet is acknowledged..:
    // pub fn send_reliable_to_and_wait(&self, msg: Message, dest: SocketAddr) -> Result<()>

    /// Blocking, with timeout (3 sec)
    pub fn recv<'a>(&mut self, buffer: &'a mut [u8]) -> Result<(SocketAddr, T), Error>
    where
        T: Deserialize<'a> + Serialize,
    {
        self.recv_internal(buffer)
    }

    fn recv_internal<'a>(&mut self, buffer: &'a mut [u8]) -> Result<(SocketAddr, T), Error>
    where
        T: Deserialize<'a> + Serialize,
    {
        // Since we may just receive an Ack, we loop until we receive an actual message
        let socket = &self.socket;
        let (_, src) = socket.recv_from(buffer)?;
        if self.connections.get(&src).is_none() {
            let conn = Connection::new(src);
            self.connections.insert(src, conn);
        }
        let conn = self.connections.get_mut(&src).unwrap();
        let packet: Packet<T> = Packet::decode(buffer)?;
        // let value = self.get_connection_or_create(src);
        let msg = conn.unwrap_message(packet, &socket)?;
        if let Some(msg) = msg {
            Ok((src, msg))
        } else {
            bail!["Ack received"]
        }
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
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use test::{black_box, Bencher};

    #[test]
    fn confirm_message_arrives() {
        let (mut client, _client_port): (Socket<bool>, _) = Socket::new_with_random_port().unwrap();
        let (mut server, server_port): (Socket<bool>, _) = Socket::new_with_random_port().unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), server_port);
        client.send_to(false, destination).unwrap();
        let mut buffer = [0u8; 5];
        server.recv(&mut buffer).unwrap();
    }

    #[test]
    fn confirm_reliable_message_arrives() {
        let (mut client, _client_port): (Socket<bool>, _) = Socket::new_with_random_port().unwrap();
        let (mut server, server_port): (Socket<bool>, _) = Socket::new_with_random_port().unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), server_port);

        client.send_reliably_to(true, destination).unwrap();
        let mut buffer = [0u8; 9];
        assert_eq![true, server.recv(&mut buffer).unwrap().1];
        assert![client.get_connection_or_create(destination).send_window[0].is_some()];
        assert![client.recv(&mut buffer).is_err()];
        assert![client.get_connection_or_create(destination).send_window[0].is_none()];
    }

    #[test]
    fn confirm_reliable_message_arrives_with_string() {
        let (mut client, _client_port): (Socket<&str>, _) = Socket::new_with_random_port().unwrap();
        let (mut server, server_port): (Socket<&str>, _) = Socket::new_with_random_port().unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), server_port);

        client.send_reliably_to("Hello World", destination).unwrap();
        let mut buffer = [0u8; 27];
        assert_eq!["Hello World", server.recv(&mut buffer).unwrap().1];
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
        let (mut client, _client_port): (Socket<u8>, _) = Socket::new_with_random_port().unwrap();
        let (mut server, server_port): (Socket<u8>, _) = Socket::new_with_random_port().unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), server_port);
        b.iter(|| {
            client
                .send_to(black_box(128u8), black_box(destination))
                .unwrap();
            let mut buffer = [0u8; 5];
            server.recv(black_box(&mut buffer)).unwrap();
        });
    }

    #[bench]
    fn allocating_1000_bytes(b: &mut Bencher) {
        b.iter(|| black_box(vec![128; 1000]));
    }

    #[bench]
    fn time_per_kilobyte(b: &mut Bencher) {
        let (mut client, _client_port): (Socket<Vec<u8>>, _) =
            Socket::new_with_random_port().unwrap();
        let (mut server, server_port): (Socket<Vec<u8>>, _) =
            Socket::new_with_random_port().unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), server_port);
        b.iter(|| {
            client
                .send_to(black_box(vec![128; 1000]), black_box(destination))
                .unwrap();
            let mut buffer = [0u8; 1012];
            server.recv(black_box(&mut buffer)).unwrap();
        });
    }

    #[bench]
    fn time_per_10_kb(b: &mut Bencher) {
        let (mut client, _client_port): (Socket<Vec<u8>>, _) =
            Socket::new_with_random_port().unwrap();
        let (mut server, server_port): (Socket<Vec<u8>>, _) =
            Socket::new_with_random_port().unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), server_port);
        b.iter(|| {
            client
                .send_to(black_box(vec![128; 10_000]), black_box(destination))
                .unwrap();
            let mut buffer = [0u8; 20_000];
            server.recv(black_box(&mut buffer)).unwrap();
        });
    }

    #[bench]
    fn time_on_realistic_load(b: &mut Bencher) {
        #[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
        struct PlayerUpdate {
            x: i32,
            y: i32,
            z: f64,
            health: Option<u32>,
        }
        let (mut client, _client_port): (Socket<PlayerUpdate>, _) =
            Socket::new_with_random_port().unwrap();
        let (mut server, server_port): (Socket<PlayerUpdate>, _) =
            Socket::new_with_random_port().unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), server_port);
        let player = PlayerUpdate {
            x: 123490,
            y: -319103,
            z: 0.341,
            health: Some(100),
        };
        b.iter(|| {
            client
                .send_to(black_box(player.clone()), black_box(destination))
                .unwrap();
            let mut buffer = [0u8; 25];
            server.recv(black_box(&mut buffer)).unwrap();
        });
    }

    #[bench]
    fn sending_tonnes_of_reliable_messages(b: &mut Bencher) {
        let (mut client, _client_port): (Socket<i32>, _) =
            Socket::new_with_random_port().unwrap();
        let (mut server, server_port): (Socket<i32>, _) =
            Socket::new_with_random_port().unwrap();

        let destination = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), server_port);
        b.iter(|| {
            client.send_reliably_to(black_box(1234567890), black_box(destination)).unwrap();
            let mut buffer = [0u8; 12];
            server.recv(black_box(&mut buffer)).unwrap();
        });
    }
}
