macro_rules! impl_try_from {
    ($name:ident; $($tail:tt)*) => {
        impl_try_from!(@inner $name; $($tail)*);
    };
    (@inner $name:ident; Primitive -> $to:ident, $($tail:tt)*) => {
        impl TryFrom<Primitive> for $to {
            type Error = Primitive;

            fn try_from(value: Primitive) -> Result<Self, Self::Error> {
                let v = value.into();
                if let Primitive::$name(val) = v {
                    Ok(val)
                } else {
                    Err(v)
                }
            }
        }
        impl_try_from!(@inner $name; $($tail)*);
    };
    (@inner $name:ident; $from:ident -> $to:ident, $($tail:tt)*) => {
        impl TryFrom<$from> for $to {
            type Error = $from;

            fn try_from(value: $from) -> Result<Self, Self::Error> {
                let v = value.into();
                if let Sexp::Primitive(Primitive::$name(val)) = v {
                    Ok(val)
                } else {
                    // No copy for Sexp, but HeapSexp will involve re-boxing.
                    Err(v.into())
                }
            }
        }
        impl_try_from!(@inner $name; $($tail)*);
    };
    (@inner $name:ident; ref $from:ident -> ref $to:ident, $($tail:tt)*) => {
        impl<'a> TryFrom<&'a $from> for &'a $to {
            type Error = &'a $from;

            fn try_from(value: &'a $from) -> Result<Self, Self::Error> {
                if let Sexp::Primitive(Primitive::$name(val)) = value {
                    Ok(val)
                } else {
                    Err(value)
                }
            }
        }
        impl_try_from!(@inner $name; $($tail)*);
    };
    (@inner $name:ident; Option<$from:ident> -> $to:ident, $($tail:tt)*) => {
        impl TryFrom<Option<$from>> for $to {
            type Error = Option<$from>;

            fn try_from(value: Option<$from>) -> Result<Self, Self::Error> {
                match value {
                    Some(v) => {
                        if let Sexp::Primitive(Primitive::$name(val)) = v {
                            Ok(val)
                        } else {
                            Err(Some(v))
                        }
                    }
                    None => Err(None)
                }
            }
        }
        impl_try_from!(@inner $name; $($tail)*);
    };
    (@inner $name:ident; Option<ref $from:ident> -> ref $to:ident, $($tail:tt)*) => {
        impl<'a> TryFrom<Option<&'a $from>> for &'a $to {
            type Error = Option<&'a $from>;

            fn try_from(value: Option<&'a $from>) -> Result<Self, Self::Error> {
                match value {
                    Some(v) => {
                        if let Sexp::Primitive(Primitive::$name(val)) = v {
                            Ok(val)
                        } else {
                            Err(Some(v))
                        }
                    }
                    None => Err(None)
                }
            }
        }
        impl_try_from!(@inner $name; $($tail)*);
    };
    (@inner $name:ident; Option<ref mut $from:ident> -> ref mut $to:ident, $($tail:tt)*) => {
        impl<'a> TryFrom<Option<&'a mut $from>> for &'a mut $to {
            type Error = Option<&'a mut $from>;

            fn try_from(value: Option<&'a mut $from>) -> Result<Self, Self::Error> {
                match value {
                    Some(v) => {
                        if let Sexp::Primitive(Primitive::$name(val)) = v {
                            Ok(val)
                        } else {
                            Err(Some(v))
                        }
                    }
                    None => Err(None)
                }
            }
        }
        impl_try_from!(@inner $name; $($tail)*);
    };
    (@inner $name:ident; Result<$from:ident> -> $to:ident, $($tail:tt)*) => {
        impl<E> TryFrom<Result<$from, E>> for $to {
            type Error = Result<$from, E>;

            fn try_from(value: Result<$from, E>) -> Result<Self, Self::Error> {
                match value {
                    Ok(v) => {
                        if let Sexp::Primitive(Primitive::$name(val)) = v {
                            Ok(val)
                        } else {
                            Err(Ok(v))
                        }
                    }
                    Err(e) => Err(Err(e))
                }
            }
        }
        impl_try_from!(@inner $name; $($tail)*);
    };
    (@inner $name:ident; Result<ref $from:ident> -> ref $to:ident, $($tail:tt)*) => {
        impl<'a, E> TryFrom<Result<&'a $from, E>> for &'a $to {
            type Error = Result<&'a $from, E>;

            fn try_from(value: Result<&'a $from, E>) -> Result<Self, Self::Error> {
                match value {
                    Ok(v) => {
                        if let Sexp::Primitive(Primitive::$name(val)) = v {
                            Ok(val)
                        } else {
                            Err(Ok(v))
                        }
                    }
                    Err(e) => Err(Err(e))
                }
            }
        }
        impl_try_from!(@inner $name; $($tail)*);
    };
    (@inner $name:ident;) => {};
}
