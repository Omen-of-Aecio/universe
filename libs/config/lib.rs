#[macro_use]

macro_rules! config {
    { struct $name:ident { $($t:tt)* } } => {
        config!{ @define $($t)* }
        config!{ @make_struct $name { $($t)* } }
    };
    
    // Make struct. Ignore substructures. These are already processesd somewhere else.
    { @make_struct $name:ident { $($x:ident : $y:ty $({ $($t:tt)* })* $(,)* )+ } } => {
        struct $name {
            $($x: $y),+
        }
    };
    // accept a sub-structure (and rest
    { @define $x:ident: $y:ident { $($t:tt)* }, $($rest:tt)* } => {
        config!{ struct $y { $($t)* } }
        config!{@define $($rest)*}
    };
    
    // The above rule, but just to accept ','
    { @define $x:ident: $y:ident { $($t:tt)* } $($rest:tt)* } => {
        config!{@define $x: $y { $($t)* }, $($rest)* }
    };
    // fields
    { @define $x:ident: $y:ty, $($rest:tt)* } => {
        config!{@define $($rest)*}
    };
    { @define $x:ident: $y:ty } => {
    };
    { @define } => {
    };
}

enum Type {
    String,
    f32,
}

enum Value {
    Num (f32),
    String (String),
}
