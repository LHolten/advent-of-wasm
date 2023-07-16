use phtm::CovariantOver;
use sea_query::{Expr, SimpleExpr};

use super::MyAlias;

pub trait Value<'t>: Copy {
    fn into_expr(self) -> SimpleExpr;

    fn add<T: Value<'t>>(self, rhs: T) -> MyAdd<Self, T> {
        MyAdd(self, rhs)
    }

    fn lt(self, rhs: i32) -> MyLt<Self> {
        MyLt(self, rhs)
    }

    fn eq<T: Value<'t>>(self, rhs: T) -> MyEq<Self, T> {
        MyEq(self, rhs)
    }

    fn not(self) -> MyNot<Self> {
        MyNot(self)
    }
}

impl<'t, A: Value<'t>, B: Value<'t>> Value<'t> for (A, B) {
    fn into_expr(self) -> SimpleExpr {
        Expr::tuple([self.0.into_expr(), self.1.into_expr()]).into()
    }
}

#[derive(Clone, Copy)]
pub struct MyIden<'t> {
    pub(super) name: MyAlias,
    pub(super) _t: CovariantOver<&'t ()>,
}

impl<'t> Value<'t> for MyIden<'t> {
    fn into_expr(self) -> SimpleExpr {
        Expr::col(self.name).into()
    }
}

#[derive(Clone, Copy)]
pub struct MyAdd<A, B>(A, B);

impl<'t, A: Value<'t>, B: Value<'t>> Value<'t> for MyAdd<A, B> {
    fn into_expr(self) -> SimpleExpr {
        self.0.into_expr().add(self.1.into_expr())
    }
}

#[derive(Clone, Copy)]
pub struct MyNot<T>(T);

impl<'t, T: Value<'t>> Value<'t> for MyNot<T> {
    fn into_expr(self) -> SimpleExpr {
        self.0.into_expr().not()
    }
}

#[derive(Clone, Copy)]
pub struct MyLt<A>(A, i32);

impl<'t, A: Value<'t>> Value<'t> for MyLt<A> {
    fn into_expr(self) -> SimpleExpr {
        Expr::expr(self.0.into_expr()).lt(self.1)
    }
}

#[derive(Clone, Copy)]
pub struct MyEq<A, B>(A, B);

impl<'t, A: Value<'t>, B: Value<'t>> Value<'t> for MyEq<A, B> {
    fn into_expr(self) -> SimpleExpr {
        self.0.into_expr().eq(self.1.into_expr())
    }
}
