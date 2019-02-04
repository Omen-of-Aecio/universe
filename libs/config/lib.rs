#[macro_use]
extern crate failure;

pub trait ConfigType {
    fn is_leaf() -> bool;
    fn set(&mut self, path: Path, value: Value) -> Result<(), failure::Error>;
    fn get_paths() -> Vec<(Path, Type)>;
}

impl ConfigType for f32 {
    fn is_leaf() -> bool {
        true
    }
    fn set(&mut self, _path: Path, value: Value) -> Result<(), failure::Error> {
        if let Value::Num(s) = value {
            *self = s;
            Ok(())
        } else {
            bail!("f32::set: wrong value type")
        }
    }
    fn get_paths() -> Vec<(Path, Type)> {
        Vec::new()
    }
}
impl ConfigType for String {
    fn is_leaf() -> bool {
        true
    }
    fn set(&mut self, _path: Path, value: Value) -> Result<(), failure::Error> {
        if let Value::String(s) = value {
            *self = s;
            Ok(())
        } else {
            bail!("f32::set: wrong value type")
        }
    }
    fn get_paths() -> Vec<(Path, Type)> {
        Vec::new()
    }
}

#[derive(Copy, Clone)]
pub enum Type {
    String,
    Num,
}

#[derive(Clone)]
pub enum Value {
    Num (f32),
    String (String),
}
impl Value {
    pub fn as_num(self) -> Result<f32, failure::Error> {
        if let Value::Num(num) = self {
            Ok(num)
        } else {
            bail!("Value::as_num: not a Num")
        }
    }
    pub fn as_string(self) -> Result<String, failure::Error> {
        if let Value::String(string) = self {
            Ok(string)
        } else {
            bail!("Value::as_strin: not a String")
        }
    }
}

#[derive(Clone)]
pub struct Path {
    nodes: Vec<String>,
}
impl Path {
    pub fn new(nodes: Vec<String>) -> Path {
        Path {
            nodes: nodes.into_iter().rev().collect()
        }
    }
    pub fn pop_front(&mut self) -> Option<String> {
        self.nodes.pop()
    }
    pub fn push_front(&mut self, value: String) {
        self.nodes.push(value)
    }
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}


// Note: makes everything pub
#[macro_export]
macro_rules! is_string {
    { String } => {true};
    { $y:ty } => {false};
}
#[macro_export]
macro_rules! is_f32 {
    { f32 } => {true};
    { $y:ty } => {false};
}


#[macro_export]
macro_rules! get_paths_recurse {
    { $x:ident : String, $paths:ident } => {
        let field_name = stringify!($x).to_string();
        $paths.push(Path::new(vec![field_name]), config::Type::String);
    };
    { $x:ident : f32, $paths:ident } => {
        let field_name = stringify!($x).to_string();
        $paths.push(Path::new(vec![field_name]), config::Type::Num);
    };
    { $x:ident : $y:ty, $paths:ident } => {
        let field_name = stringify!($x).to_string();
            for (mut path, ty) in <$y>::get_paths() {
                path.push_front(field_name.clone());
                $paths.push((path, ty));
            }
    };
}


// NOTE: ignore the $(#[$($m:meta)*])* and corresponding $(#[$($m)*])* when reading. These are just
// to pass meta items to ALL struct definitions.
#[macro_export]
macro_rules! config {
    { $(#[$($m:meta)*])* struct $name:ident { $($t:tt)* } } => {
        config!{ @define $(#[$($m)*])* $($t)* }
        config!{ @make_struct $(#[$($m)*])* $name { $($t)* } }
    };
    
    // Make struct. Ignore substructures. These are already processesd somewhere else.
    { @make_struct $(#[$($m:meta)*])* $name:ident { $($x:ident : $y:ty $({ $($t:tt)* })* $(,)* )+ } } => {
        $(#[$($m)*])*
        pub struct $name {
            $(pub $x: $y),+
        }
        impl config::ConfigType for $name {
            fn is_leaf() -> bool {false}
            fn set(&mut self, mut path: config::Path, value: config::Value) -> Result<(), failure::Error> {
                use failure::bail;
                // TODO/NOTE: path could also easily be &mut if that performs better

                match path.pop_front() {
                    Some(field) => {
                        $(
                        if field == stringify!($x) {
                            self.$x.set(path.clone(), value.clone())?;
                        }
                        // TODO: else-if, and else: error
                        )*
                    },
                    None => bail!("Error in path"),
                }
                Ok(())
            }
            fn get_paths() -> Vec<(config::Path, config::Type)> {
                let mut paths = Vec::new();
                $( {
                    get_paths_recurse!($x: $y, paths);
                } )*
                paths
            }
        }
    };

    // accept a sub-structure (and rest)
    { @define $(#[$($m:meta)*])* $x:ident: $y:ident { $($t:tt)* }, $($rest:tt)* } => {
        config!{$(#[$($m)*])* struct $y { $($t)* } }
        config!{@define $(#[$($m)*])* $($rest)*}
    };
    
    // The above rule, but just to accept ','
    { @define $(#[$($m:meta)*])* $x:ident: $y:ident { $($t:tt)* } $($rest:tt)* } => {
        config!{@define $(#[$($m)*])* $x: $y { $($t)* }, $($rest)* }
    };
    // fields
    { @define $(#[$($m:meta)*])* $x:ident: $y:ty, $($rest:tt)* } => {
        config!{@define $(#[$($m)*])* $($rest)*}
    };
    { @define $(#[$($m:meta)*])* $x:ident: $y:ty } => {
    };
    { @define $(#[$($m:meta)*])* } => {
    };
}

