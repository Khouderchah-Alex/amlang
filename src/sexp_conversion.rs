/// Breaks a Sexp into Result<tuple of component types, EvalErr>, assuming all
/// component types implement TryFrom<Sexp>.
///
/// Optional remainder at end is an arbitrary identifier and cannot accept
/// repetitions. Will return as final tuple element of type Option<HeapSexp>.
macro_rules! break_by_types {
    (@ignore $_ignored:ident) => {};
    ($sexp:expr, $($type:ident),+ $(;$remainder:tt),*) => {
        {
            match $sexp {
                Sexp::Primitive(primitive) => {
                    Err(crate::function::EvalErr::InvalidSexp(primitive.clone().into()))
                }
                Sexp::Cons(cons) => {
                    let mut iter = cons.into_iter();
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
                                    Some(sexp) =>  {
                                        if let Ok(_) = <&$type>::try_from(&*sexp) {
                                            i += 1;
                                            <$type>::try_from(*sexp).unwrap()
                                        } else {
                                            return Err(crate::function::EvalErr::InvalidArgument{
                                                given: *sexp.clone(),
                                                expected: std::borrow::Cow::Owned(
                                                    "type ".to_string() + stringify!($type)
                                                ),
                                            });
                                        }
                                    }
                                    None =>  {
                                        return Err(crate::function::EvalErr::WrongArgumentCount{
                                            given: i,
                                            expected: crate::function::ExpectedCount::Exactly(
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
                            iter = Cons::default().into_iter();
                        )*
                        if let Some(_) = iter.next() {
                            return Err(crate::function::EvalErr::WrongArgumentCount{
                                given: i + 1 + iter.count(),
                                expected: crate::function::ExpectedCount::Exactly(i),
                            });
                        }
                        ret
                    };

                    tuple()
                }
            }

        }
    };
}


#[cfg(test)]
#[path = "./sexp_conversion_test.rs"]
mod sexp_conversion_test;
