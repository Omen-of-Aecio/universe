use logger::Generic;
use geometry::{vec::Vec2};

#[derive(Clone)]
pub enum Log {
    Bool(&'static str, &'static str, bool),
    Char(&'static str, &'static str, char),
    Coordinates(Vec2, Vec2),
    Dynamic(String),
    I64(&'static str, &'static str, i64),
    Static(&'static str),
    StaticDynamic(&'static str, &'static str, String),
    StaticDynamics(&'static str, Vec<(&'static str, String)>),
    U64(&'static str, &'static str, u64),
    U8(&'static str, &'static str, u8),
    Usize(&'static str, &'static str, usize),
    Usize2(&'static str, &'static str, usize, &'static str, usize),
    Generic(Generic),
}

impl From<Generic> for Log {
    fn from(data: Generic) -> Self {
        Log::Generic(data)
    }
}

impl From<&'static str> for Log {
    fn from(data: &'static str) -> Self {
        Log::Static(data)
    }
}

impl std::fmt::Display for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Log::Bool(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::Char(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::Coordinates(world, mouse) => {
                write![f, "Mouse on screen, world={:?}, mouse={:?}", world, mouse]
            }
            Log::Dynamic(str) => write![f, "{}", str],
            Log::Generic(handle) => handle.fmt(f),
            Log::I64(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::Static(str) => write![f, "{}", str],
            Log::StaticDynamic(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::StaticDynamics(msg, kvs) => {
                write![f, "{}", msg]?;
                if !kvs.is_empty() {
                    write![f, ", "]?;
                    for kv in kvs.iter().take(kvs.len() - 1) {
                        write![f, "{}={}, ", kv.0, kv.1]?;
                    }
                    let kv = kvs.last().unwrap();
                    write![f, "{}={}", kv.0, kv.1]?;
                }
                Ok(())
            }
            Log::U8(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::U64(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::Usize(msg, key, value) => write![f, "{}, {}={}", msg, key, value],
            Log::Usize2(msg, key, value, key2, value2) => {
                write![f, "{}, {}={}, {}={}", msg, key, value, key2, value2]
            }
        }
    }
}

