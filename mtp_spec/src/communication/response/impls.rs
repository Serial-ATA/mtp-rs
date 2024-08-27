const fn counter<const N: usize>(_: [(); N]) -> usize { N }

macro_rules! replace_expr {
    ($_t:tt $sub:expr) => { $sub }
}

const MAX_PARAMETERS: usize = 5;

macro_rules! define_response {
	(
		$(#[$meta:meta])*
		pub struct $name:ident {
			code: $code:expr,
			parameters: ($($param:ident: $ty:ty),* $(,)?),
		}
	) => {
		const _: usize = {
			if count_helper([$(replace_expr!($param ())),*]) > MAX_PARAMETERS {
				panic!("Too many parameters");
			}
		}

		$(#[$meta])*
		pub struct $name {
			parameters: [Option<Parameter>; count_helper([$(replace_expr!($param ())),*])],
		}

		impl $name {
			const OPCODE: u16 = $code;

			paste::paste! {
				pub fn new($($param: $ty),*) -> Self {
					Self {
						parameters: [$($param),*],
					}
				}
			}
		}
	}
}