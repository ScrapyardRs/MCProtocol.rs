pub mod blocks;
pub mod chunk;
pub mod codec;

enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L, R> Either<L, R> {
    pub fn assert_left(self) -> L {
        match self {
            Either::Left(left) => left,
            Either::Right(_) => unreachable!(),
        }
    }

    pub fn assert_right(self) -> R {
        match self {
            Either::Left(_) => unreachable!(),
            Either::Right(right) => right,
        }
    }
}
