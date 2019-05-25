/// Equivalent to logging to the [log] function with an appropriate level, context, and a
/// [Generic].
#[macro_export]
macro_rules! log {
    ($n:expr, $log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?; $($key:expr => $val:expr),* $(,)?; clone $($cl:ident),* $(,)?) => {{
        $(
            let $cl = $cl.clone();
        )*
        $log.log($n, $ctx, $crate::make_generic__(::std::sync::Arc::new(move |f| -> ::std::fmt::Result {
            Ok({
                write![f, $fmt, $($msg),*]?;
                $(
                    write![f, ", {}={}", $key, $val]?;
                )*
            })
        })))
    }};
    ($n:expr, $log:expr, $ctx:expr, $fmt:expr $(,)?; $($key:expr => $val:expr),* $(,)?; clone $($cl:ident),* $(,)?) => {{
        $(
            let $cl = $cl.clone();
        )*
        $log.log($n, $ctx, $crate::make_generic__(::std::sync::Arc::new(move |f| -> ::std::fmt::Result {
            Ok({
                write![f, $fmt]?;
                $(
                    write![f, ", {}={}", $key, $val]?;
                )*
            })
        })))
    }};
    ($n:expr, $log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?; $($key:expr => $val:expr),* $(,)?) => {{
        $log.log($n, $ctx, $crate::make_generic__(::std::sync::Arc::new(move |f| -> ::std::fmt::Result {
            Ok({
                write![f, $fmt, $($msg),*]?;
                $(
                    write![f, ", {}={}", $key, $val]?;
                )*
            })
        })))
    }};
    ($n:expr, $log:expr, $ctx:expr, $fmt:expr; $($key:expr => $val:expr),* $(,)?) => {{
        $log.log($n, $ctx, $crate::make_generic__(::std::sync::Arc::new(move |f| -> ::std::fmt::Result {
            Ok({
                write![f, $fmt]?;
                $(
                    write![f, ", {}={}", $key, $val]?;
                )*
            })
        })))
    }};
    ($n:expr, $log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?) => {{
        $log.log($n, $ctx, $crate::make_generic__(::std::sync::Arc::new(move |f| -> ::std::fmt::Result {
            write![f, $fmt, $($msg),*]
        })))
    }};
    ($n:expr, $log:expr, $ctx:expr, $fmt:expr $(,)?) => {{
        $log.log($n, $ctx, $crate::make_generic__(::std::sync::Arc::new(move |f| -> ::std::fmt::Result {
            write![f, $fmt]
        })))
    }};
}

/// Equivalent to [log!] with a level of 255
#[macro_export]
macro_rules! trace {
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?; $($key:expr => $val:expr),* $(,)?; clone $($cl:ident),* $(,)?) => { $crate::log![255, $log, $ctx, $fmt, $($msg),*; $($key => $val),*; clone $($cl),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?; $($key:expr => $val:expr),* $(,)?; clone $($cl:ident),* $(,)?) => { $crate::log![255, $log, $ctx, $fmt; $($key => $val),*; clone $($cl),*] };
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?; $($key:expr => $val:expr),* $(,)?) => { $crate::log![255, $log, $ctx, $fmt, $($msg),*; $($key => $val),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?; $($key:expr => $val:expr),* $(,)?) => { $crate::log![255, $log, $ctx, $fmt; $($key => $val),*] };
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?) => { $crate::log![255, $log, $ctx, $fmt, $($msg),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?) => { $crate::log![255, $log, $ctx, $fmt] };
}

/// Equivalent to [log!] with a level of 192
#[macro_export]
macro_rules! debug {
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?; $($key:expr => $val:expr),* $(,)?; clone $($cl:ident),* $(,)?) => { $crate::log![192, $log, $ctx, $fmt, $($msg),*; $($key => $val),*; clone $($cl),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?; $($key:expr => $val:expr),* $(,)?; clone $($cl:ident),* $(,)?) => { $crate::log![192, $log, $ctx, $fmt; $($key => $val),*; clone $($cl),*] };
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?; $($key:expr => $val:expr),* $(,)?) => { $crate::log![192, $log, $ctx, $fmt, $($msg),*; $($key => $val),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?; $($key:expr => $val:expr),* $(,)?) => { $crate::log![192, $log, $ctx, $fmt; $($key => $val),*] };
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?) => { $crate::log![192, $log, $ctx, $fmt, $($msg),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?) => { $crate::log![192, $log, $ctx, $fmt] };
}

/// Equivalent to [log!] with a level of 128
#[macro_export]
macro_rules! info {
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?; $($key:expr => $val:expr),* $(,)?; clone $($cl:ident),* $(,)?) => { $crate::log![128, $log, $ctx, $fmt, $($msg),*; $($key => $val),*; clone $($cl),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?; $($key:expr => $val:expr),* $(,)?; clone $($cl:ident),* $(,)?) => { $crate::log![128, $log, $ctx, $fmt; $($key => $val),*; clone $($cl),*] };
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?; $($key:expr => $val:expr),* $(,)?) => { $crate::log![128, $log, $ctx, $fmt, $($msg),*; $($key => $val),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?; $($key:expr => $val:expr),* $(,)?) => { $crate::log![128, $log, $ctx, $fmt; $($key => $val),*] };
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?) => { $crate::log![128, $log, $ctx, $fmt, $($msg),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?) => { $crate::log![128, $log, $ctx, $fmt] };
}

/// Equivalent to [log!] with a level of 64
#[macro_export]
macro_rules! warn {
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?; $($key:expr => $val:expr),* $(,)?; clone $($cl:ident),* $(,)?) => { $crate::log![64, $log, $ctx, $fmt, $($msg),*; $($key => $val),*; clone $($cl),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?; $($key:expr => $val:expr),* $(,)?; clone $($cl:ident),* $(,)?) => { $crate::log![64, $log, $ctx, $fmt; $($key => $val),*; clone $($cl),*] };
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?; $($key:expr => $val:expr),* $(,)?) => { $crate::log![64, $log, $ctx, $fmt, $($msg),*; $($key => $val),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?; $($key:expr => $val:expr),* $(,)?) => { $crate::log![64, $log, $ctx, $fmt; $($key => $val),*] };
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?) => { $crate::log![64, $log, $ctx, $fmt, $($msg),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?) => { $crate::log![64, $log, $ctx, $fmt] };
}

/// Equivalent to [log!] with a level of 0
#[macro_export]
macro_rules! error {
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?; $($key:expr => $val:expr),* $(,)?; clone $($cl:ident),* $(,)?) => { $crate::log![0, $log, $ctx, $fmt, $($msg),*; $($key => $val),*; clone $($cl),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?; $($key:expr => $val:expr),* $(,)?; clone $($cl:ident),* $(,)?) => { $crate::log![0, $log, $ctx, $fmt; $($key => $val),*; clone $($cl),*] };
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?; $($key:expr => $val:expr),* $(,)?) => { $crate::log![0, $log, $ctx, $fmt, $($msg),*; $($key => $val),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?; $($key:expr => $val:expr),* $(,)?) => { $crate::log![0, $log, $ctx, $fmt; $($key => $val),*] };
    ($log:expr, $ctx:expr, $fmt:expr, $($msg:expr),* $(,)?) => { $crate::log![0, $log, $ctx, $fmt, $($msg),*] };
    ($log:expr, $ctx:expr, $fmt:expr $(,)?) => { $crate::log![0, $log, $ctx, $fmt] };
}
