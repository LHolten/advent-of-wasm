use phtm::{CovariantOver, PhantomData};
use sea_query::SimpleExpr;

use super::value::Value;

pub struct DynRow<'t> {
    list: Vec<SimpleExpr>,
    _t: CovariantOver<&'t ()>,
}

pub trait Row<'t>: Copy {
    fn into_row(&self) -> Vec<SimpleExpr>;
    fn into_dyn(&self) -> DynRow<'_> {
        DynRow {
            list: self.into_row(),
            _t: PhantomData,
        }
    }
    // fn from_row(row: Vec<MyIden<'t>>) -> Self::Target<'t>;
}

impl<'t, T: Value<'t>> Row<'t> for T {
    fn into_row(&self) -> Vec<SimpleExpr> {
        vec![self.into_expr()]
    }
}

impl<'t, A: Row<'t>, B: Row<'t>> Row<'t> for (A, B) {
    fn into_row(&self) -> Vec<SimpleExpr> {
        let mut res = self.0.into_row();
        res.extend(self.1.into_row());
        res
    }
}

#[derive(Default, Clone, Copy)]
pub struct Empty;

impl<'t> Row<'t> for Empty {
    fn into_row(&self) -> Vec<SimpleExpr> {
        vec![]
    }
}
