//! Note on snapshots:
//! Snapshots are incremental: only that which has changed is sent to clients.
//! Only upon explicit request (or join) of a client does the client
//! receive a complete snapshot. This snapshot should be transmitted reliably.
use geometry::vec::Vec2;
use component::*;
use serde::{Serialize, Serializer, Deserialize, Deserializer, de::Visitor};
use std::fmt;
use std::collections::BTreeMap;
use std::marker::PhantomData;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Message {
    // Messages from server
    Welcome {width: u32, height: u32, you: u32, white_base: Vec2, black_base: Vec2},
    WorldRect {x: usize, y: usize, width: usize, pixels: Vec<u8>},
    State (Snapshot),

    Players (Vec<SrvPlayer>),
    NewPlayer (SrvPlayer),

    // Messages from client
    Join {snapshot_rate: f32},
    Input (PlayerInput),
    ToggleGravity,
    BulletFire { direction: Vec2 },
}


// TODO old..
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SrvPlayer {
    pub id: u32,
    pub col: Color,
    pub pos: Vec2,
}
impl SrvPlayer {
    pub fn new(player: &Player, col: Color, pos: &Pos) -> SrvPlayer {
        SrvPlayer {
            id: player.id,
            col: col,
            pos: pos.transl,
        }
    }
}



// TAKE TWO

/// Incremental or entire snapshot of state at the server, to be sent to clients
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Snapshot {
    /// Hash map from (ID, type) to a list of components for this entity. If the value is None, it
    /// means deletion of this entity
    pub entities: BTreeMap<u32, Option<Entity>>,
}

/// Incremental or entire representation of an Entity
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entity{
    pub ty: Type,
    pub components: Components,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Type {Player, Bullet}



// TODO: derive getters for the fields, that return Option?
//       ... or derive the whole struct with a macro/derive...
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Components ( SerOption<Pos>, SerOption<Vel>, SerOption<Shape>, SerOption<Color>);
impl Components {
    pub fn new(a: &Pos, b: &Vel, c: &Shape, d: &Color) -> Components {
        Components (SerOption::new(a), SerOption::new(b), SerOption::new(c), SerOption::new(d))
    }
    /// Copy over the components that are marked as present in the other Components
    pub fn update(&mut self, other: &Components) {
        // TODO automatize
        if other.0.present { self.0 = other.0.clone(); }
        if other.1.present { self.1 = other.1.clone(); }
        if other.2.present { self.2 = other.2.clone(); }
        if other.3.present { self.3 = other.3.clone(); }
        // for each field, set SerOption::serialize to `true` if the two versions differ
        // and to `false` if they are the same.
    }
}


/// Serializes as `Some(T)` if `self.serialize == true`. Otherwise as `None`.
#[derive(Clone, Debug, Default)]
pub struct SerOption<T: Default> {
    pub present: bool,
    pub data: T,
}
impl<T: Default + Clone> SerOption<T> {
    pub fn new(data: &T) -> SerOption<T> {
        SerOption {
            present: true,
            data: data.clone(),
        }
    }
}



// (de)serialization:


impl<T: Serialize + Default> Serialize for SerOption<T> {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        if self.present {
            ser.serialize_some(&self.data)
        } else {
            ser.serialize_none()
        }
    }
}
impl<'de, T: Default + Deserialize<'de>> Deserialize<'de> for SerOption<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_option(SerOptionVisitor::new())
    }
}

struct SerOptionVisitor<T> {
    phantom: PhantomData<T>,
}
impl<T> SerOptionVisitor<T> {
    pub fn new() -> SerOptionVisitor<T> {
        SerOptionVisitor {
            phantom: PhantomData,
        }
    }
}
impl<'de, T: Default> Visitor<'de> for SerOptionVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = SerOption<T>;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "SerOption")
    }
    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(SerOption::default())
    }
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where D: Deserializer<'de> {
        Ok(SerOption {
            present: true,
            data: T::deserialize(deserializer)?,
        })
    }
}
