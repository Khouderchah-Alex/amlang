/// Breaks a HeapSexp into Result<tuple of component types, LangErr>,
/// assuming all component types implement TryFrom<$type> for Sexp.
///
/// Optional remainder accepts an arbitrary identifier and append an
/// Option<HeapSexp> to the end of the result tuple.
///
/// If available, state can be passed to make errors stateful.
/// Note that clients using a Sexp must manually call HeapSexp::new(_).
///
/// Example:
///  let (a, b, tail) = break_hsexp!(original => (Symbol, HeapSexp; remainder), self.state())?;
macro_rules! break_hsexp {
    (@ignore $_ignored:ident) => {};
    ($sexp:expr => ($($type:ident),+ $(;$remainder:tt)?) $(,$state:expr)?) => {
        {
            // Generate stateful or stateless error depending on existence of $state.
            let err = |kind| {
                $(
                    return Err(crate::lang_err::LangErr::with_state(
                        $state.clone(),
                        kind
                    ));
                )*
                #[allow(unreachable_code)]
                Err(crate::lang_err::LangErr::empty_state(kind))
            };
            let mut iter = $sexp.into_iter();
            let tuple = || {
                let mut expected: usize = 0;
                $(
                    break_hsexp!(@ignore $type);
                    expected += 1;
                )+
                let mut i: usize = 0;
                let ret = Ok((
                    $(
                        match iter.next() {
                            Some((sexp, from_cons)) =>  {
                                if !from_cons {
                                    return err(crate::lang_err::ErrKind::InvalidSexp(sexp.into()));
                                }
                                match <$type>::try_from(*sexp) {
                                    Ok(val) => {
                                        i += 1;
                                        val
                                    },
                                    Err(original) => {
                                        return err(crate::lang_err::ErrKind::InvalidArgument{
                                            given: original.into(),
                                            expected: std::borrow::Cow::Owned(
                                                "type ".to_string() + stringify!($type)
                                            ),
                                        });
                                    }
                                }
                            }
                            None =>  {
                                return err(crate::lang_err::ErrKind::WrongArgumentCount{
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
                            break_hsexp!(@ignore $remainder);
                            iter.consume()
                        }
                    )*
                ));

                $(
                    break_hsexp!(@ignore $remainder);
                    iter = crate::sexp::SexpIntoIter::default();
                )*
                if let Some(_) = iter.next() {
                    return err(crate::lang_err::ErrKind::WrongArgumentCount{
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
    (@cons $car:expr, $cdr:expr) => {
        <crate::sexp::Sexp>::from(
            crate::sexp::Cons::new($car.into(), $cdr.into()))
    };
    (($elem:expr, $($sub_tail:tt)*), $($tail:tt)*) => {
        {
            list_inner!(@cons
                  list_inner!(@cons
                    Some(crate::sexp::HeapSexp::new($elem.into())),
                    list_inner!($($sub_tail)*)),
                list_inner!($($tail)*))
        }
    };
    ($elem:expr, $($tail:tt)*) => {
        {
            list_inner!(@cons
                Some(crate::sexp::HeapSexp::new($elem.into())),
                list_inner!($($tail)*))
        }
    };
}

/// Returns the elements as a Sexp list.
///
/// Provided Primitive elements must implement Into<Sexp>.
/// Trailing commas currently must be used.
///
/// Example:
///   list!(a, b, (c, (d)), e)
macro_rules! list {
    ($($tail:tt)*) => {
        list_inner!($($tail)*)
    }
}


#[cfg(test)]
#[path = "./sexp_conversion_test.rs"]
mod sexp_conversion_test;
