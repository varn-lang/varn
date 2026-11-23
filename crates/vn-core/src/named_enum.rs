#[macro_export]
macro_rules! named_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $( $(#[$vmeta:meta])* $variant:ident $(= $disc:expr)? => $text:expr ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis enum $name {
            $( $(#[$vmeta])* $variant $(= $disc)?, )*
        }

        impl $name {
            pub const COUNT: usize = [$(stringify!($variant)),*].len();

            pub const ALL: [Self; Self::COUNT] = [$(Self::$variant),*];

            pub const fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)*
                }
            }

            pub fn of(name: &str) -> ::core::option::Option<Self> {
                Self::ALL.into_iter().find(|v| v.name() == name)
            }
        }
    };
}
