/// Breaks a Sexp into Result<tuple of component types, LangErr>, assuming all
/// component types implement TryFrom<$type> for Sexp.
///
/// Optional remainder accepts an arbitrary identifier and append an
/// Option<HeapSexp> to the end of the result tuple.
macro_rules! break_by_types {
    (@ignore $_ignored:ident) => {};
    ($sexp:expr, $($type:ident),+ $(;$remainder:tt)?) => {
        {
            let mut iter = $sexp.into_iter();
            let tuple = || {
                let mut expected: usize = 0;
                $(
                    break_by_types!(@ignore $type);
                    expected += 1;
                )+
                let mut i: usize = 0;
                let ret = Ok((
                    $(
                        match iter.next() {
                            Some((sexp, from_cons)) =>  {
                                if !from_cons {
                                    return err_nost!(InvalidSexp(sexp.into()));
                                }
                                match <$type>::try_from(*sexp) {
                                    Ok(val) => {
                                        i += 1;
                                        val
                                    },
                                    Err(original) => {
                                        return err_nost!(InvalidArgument{
                                            given: original.into(),
                                            expected: std::borrow::Cow::Owned(
                                                "type ".to_string() + stringify!($type)
                                            ),
                                        });
                                    }
                                }
                            }
                            None =>  {
                                return err_nost!(WrongArgumentCount{
                                    given: i,
                                    expected: crate::lang_err::ExpectedCount::Exactly(
                                        expected
                                    ),
                                });
                            }
                        },
                    )+
                    $(
                        {
                            break_by_types!(@ignore $remainder);
                            iter.consume()
                        }
                    )*
                ));

                $(
                    break_by_types!(@ignore $remainder);
                    iter = Sexp::default().into_iter();
                )*
                if let Some(_) = iter.next() {
                    return err_nost!(WrongArgumentCount{
                        given: i + 1 + iter.count(),
                        expected: crate::lang_err::ExpectedCount::Exactly(i),
                    });
                }
                ret
            };

            tuple()
        }
    };
}

// Should not be used directly. Use list! below.
macro_rules! list_inner {
    () => { None };
    (($elem:expr, $($sub_tail:tt)*), $($tail:tt)*) => {
        {
            crate::sexp::cons(
                crate::sexp::cons(
                    Some(crate::sexp::HeapSexp::new($elem.into())),
                    list_inner!($($sub_tail)*)),
                list_inner!($($tail)*))
        }
    };
    ($elem:expr, $($tail:tt)*) => {
        {
            crate::sexp::cons(
                Some(crate::sexp::HeapSexp::new($elem.into())),
                list_inner!($($tail)*))
        }
    };
}

/// Returns the specified sexp as a HeapSexp.
///
/// Provided Primitive elements must implement Into<Sexp>.
/// Trailing commas currently must be used.
macro_rules! list {
    ($($tail:tt)*) => {
        list_inner!($($tail)*).unwrap()
    }
}


#[cfg(test)]
#[path = "./sexp_conversion_test.rs"]
mod sexp_conversion_test;
