use phtm::{CovariantOver, PhantomData};
use sea_query::SimpleExpr;

use super::value::MyIden;

pub struct TableRef<'a, 't> {
    pub(super) callback: &'a dyn FnMut(&'static str) -> MyIden<'t>,
}

impl<'a, 't> TableRef<'a, 't> {
    pub fn get(&self, name: &'static str) -> MyIden<'t> {
        todo!()
    }
}

pub trait Table<'t>: Copy {
    const NAME: &'static str;
    fn from_table(t: TableRef<'_, 't>) -> Self;
    fn into_row(&self) -> Vec<SimpleExpr>;
    // fn from_row(row: Vec<MyIden<'t>>) -> Self::Target<'t>;
}
