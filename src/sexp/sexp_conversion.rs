/// Breaks a Sexp, HeapSexp, or Sexp iter of some kind into Result<tuple of
/// component types, Error>, assuming all component types implement
/// TryFrom<$type> for Sexp.
///
/// Optional remainder accepts an arbitrary identifier and append an
/// Option<HeapSexp> to the end of the result tuple.
///
/// If available, agent can be passed to make errors stateful.
///
/// Example:
///  let (a, b, tail) = break_sexp!(original => (Symbol, HeapSexp; remainder), self.agent())?;
// TODO(func) Have remainder return iter so that other Iterators can be used.
#[macro_export]
macro_rules! break_sexp {
    (@ignore $_ignored:ty) => {};
    ($sexp:expr => ($($type:ty),* $(;$remainder:tt)?) $(,$agent:expr)?) => {
        {
            // Generate stateful or stateless error depending on existence of $agent.
            let err = |kind| {
                $(
                    return Err($crate::error::Error::with_cont(
                        $agent.exec_state().clone(),
                        Box::new(kind)
                    ));
                )*
                #[allow(unreachable_code)]
                Err($crate::error::Error::no_cont(kind))
            };
            let mut iter = $sexp.into_iter();
            let tuple = || {
                // Ignore warnings for case with empty component tuple.
                #[allow(unused_mut, unused_variables)]
                let mut expected: usize = 0;
                $(
                    break_sexp!(@ignore $type);
                    expected += 1;
                )*
                // Ignore warnings for case with empty component tuple.
                #[allow(unused_mut, unused_variables)]
                let mut i: usize = 0;
                let ret = Ok((
                    $(
                        match iter.next() {
                            Some((sexp, proper)) =>  {
                                if !proper {
                                    return err($crate::agent::lang_error::LangError::InvalidSexp(
                                        // TODO(perf) Avoid clone for non-ref types.
                                        sexp.clone().into())
                                    );
                                }
                                match <$type as std::convert::TryFrom<_>>::try_from(sexp) {
                                    Ok(val) => {
                                        i += 1;
                                        val
                                    },
                                    Err(original) => {
                                        return err(
                                            $crate::agent::lang_error::LangError::InvalidArgument{
                                                // TODO(perf) Avoid clone for non-ref types.
                                                given: original.clone().into(),
                                                expected: std::borrow::Cow::Owned(
                                                    "type ".to_string() + stringify!($type)
                                                ),
                                            });
                                    }
                                }
                            }
                            None =>  {
                                return err($crate::agent::lang_error::LangError::WrongArgumentCount{
                                    given: i,
                                    expected: $crate::agent::lang_error::ExpectedCount::Exactly(
                                        expected
                                    ),
                                });
                            }
                        },
                    )*
                    $(
                        {
                            break_sexp!(@ignore $remainder);
                            iter.consume()
                        }
                    )*
                ));

                $(
                    break_sexp!(@ignore $remainder);
                    iter = Default::default();
                )*
                if let Some(_) = iter.next() {
                    return err($crate::agent::lang_error::LangError::WrongArgumentCount{
                        given: i + 1 + iter.count(),
                        expected: $crate::agent::lang_error::ExpectedCount::Exactly(i),
                    });
                }
                ret
            };

            tuple()
        }
    };
}

/// Returns the elements as a Sexp list.
///
/// Provided Primitive elements must implement Into<Sexp>.
///
/// Example:
///   list!(a, b, (c, (d)), e)
#[macro_export]
macro_rules! list {
    (@cons $car:expr, $cdr:expr) => {
        <$crate::sexp::Sexp>::from(
            $crate::sexp::Cons::new($car, $cdr))
    };
    (@inner) => { None as Option<$crate::sexp::HeapSexp> };
    (@inner (() $(, $($sub_tail:tt)*)?) $(, $($tail:tt)*)?) => {
        {
            list!(@cons
                  list!(@cons
                    Some($crate::sexp::HeapSexp::new(list!(@cons None, None))),
                    list!(@inner $($($sub_tail)*)*)),
                list!(@inner $($($tail)*)*))
        }
    };
    (@inner () $(, $($tail:tt)*)?) => {
        {
            list!(@cons
                Some($crate::sexp::HeapSexp::new(list!(@cons None, None))),
                list!(@inner $($($tail)*)*))
        }
    };
    (@inner ($elem:expr $(, $($sub_tail:tt)*)?) $(, $($tail:tt)*)?) => {
        {
            list!(@cons
                  list!(@cons
                    Some($crate::sexp::HeapSexp::new($elem.into())),
                    list!(@inner $($($sub_tail)*)*)),
                list!(@inner $($($tail)*)*))
        }
    };
    (@inner $elem:expr $(, $($tail:tt)*)?) => {
        {
            list!(@cons
                Some($crate::sexp::HeapSexp::new($elem.into())),
                list!(@inner $($($tail)*)*))
        }
    };
    ($($tail:tt)*) => {
        list!(@inner $($tail)*)
    };
}


#[cfg(test)]
#[path = "./sexp_conversion_test.rs"]
mod sexp_conversion_test;
