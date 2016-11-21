#![macro_use]
pub type Tile = u8;

#[macro_export]
macro_rules! time_ns {
	($($e:tt)*) => {
		{
			use time;
			let begin = time::precise_time_ns();
			let temporary = $($e)*;
			let end = time::precise_time_ns();
			(end - begin, temporary)
		}
	};
}

#[macro_export]
macro_rules! prof {
	($s:expr, $($e:tt)*) => {
		{
			let (elapsed, result) = time_ns![$($e)*];
			trace![$s; "nanos" => elapsed];
			result
		}
	};
}
