use phtm::CovariantOverLt;
use sea_query::{Expr, SimpleExpr};
use std::ops::{Add, Not};

use super::MyAlias;

pub trait Value<'t>: Copy {
    fn into_expr(self) -> SimpleExpr;

    fn add<T: Value<'t>>(self, rhs: T) -> MyAdd<Self, T> {
        MyAdd(self, rhs)
    }

    fn lt(self, arg: i32) -> Self {
        todo!()
    }

    fn eq(self, other: Self) -> Self {
        todo!()
    }

    fn not(self) -> MyNot<Self> {
        MyNot(self)
    }
}

#[derive(Clone, Copy)]
pub struct MyIden<'t> {
    name: MyAlias,
    _t: CovariantOverLt<'t>,
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
