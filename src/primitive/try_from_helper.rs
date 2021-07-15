macro_rules! impl_try_from {
    ($from:ident, $to:ident, $name:ident; $($tail:tt)*) => {
        impl TryFrom<$from> for $to {
            type Error = ();

            fn try_from(value: $from) -> Result<Self, Self::Error> {
                impl_try_from!(@base value $name)
            }
        }
        impl_try_from!($($tail)*);
    };
    (ref $from:ident, ref $to:ident, $name:ident; $($tail:tt)*) => {
        impl<'a> TryFrom<&'a $from> for &'a $to {
            type Error = ();

            fn try_from(value: &'a $from) -> Result<Self, Self::Error> {
                impl_try_from!(@base value $name)
            }
        }
        impl_try_from!($($tail)*);
    };
    (Option<$from:ident>, $to:ident, $name:ident; $($tail:tt)*) => {
        impl TryFrom<Option<$from>> for $to {
            type Error = ();

            fn try_from(value: Option<$from>) -> Result<Self, Self::Error> {
                if let Some(v) = value{
                    impl_try_from!(@base v $name)
                } else {
                    Err(())
                }
            }
        }
        impl_try_from!($($tail)*);
    };
    (Option<ref $from:ident>, ref $to:ident, $name:ident; $($tail:tt)*) => {
        impl<'a> TryFrom<Option<&'a $from>> for &'a $to {
            type Error = ();

            fn try_from(value: Option<&'a $from>) -> Result<Self, Self::Error> {
                if let Some(v) = value{
                    impl_try_from!(@base v $name)
                } else {
                    Err(())
                }
            }
        }
        impl_try_from!($($tail)*);
    };
    (Option<ref mut $from:ident>, ref mut $to:ident, $name:ident; $($tail:tt)*) => {
        impl<'a> TryFrom<Option<&'a mut $from>> for &'a mut $to {
            type Error = ();

            fn try_from(value: Option<&'a mut $from>) -> Result<Self, Self::Error> {
                if let Some(v) = value{
                    impl_try_from!(@base v $name)
                } else {
                    Err(())
                }
            }
        }
        impl_try_from!($($tail)*);
    };
    (Result<$from:ident>, $to:ident, $name:ident; $($tail:tt)*) => {
        impl<E> TryFrom<Result<$from, E>> for $to {
            type Error = ();

            fn try_from(value: Result<$from, E>) -> Result<Self, Self::Error> {
                if let Ok(v) = value{
                    impl_try_from!(@base v $name)
                } else {
                    Err(())
                }
            }
        }
        impl_try_from!($($tail)*);
    };
    (Result<ref $from:ident>, ref $to:ident, $name:ident; $($tail:tt)*) => {
        impl<'a, E> TryFrom<Result<&'a $from, E>> for &'a $to {
            type Error = ();

            fn try_from(value: Result<&'a $from, E>) -> Result<Self, Self::Error> {
                if let Ok(v) = value{
                    impl_try_from!(@base v $name)
                } else {
                    Err(())
                }
            }
        }
        impl_try_from!($($tail)*);
    };
    (@base $v:ident $name:ident) => {
        if let Sexp::Primitive(Primitive::$name(val)) = $v {
            Ok(val)
        } else {
            Err(())
        }
    };
    () => {};
}
