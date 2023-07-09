use sea_query::SimpleExpr;

use super::value::MyIden;

pub struct TableRef<'a, 't> {
    pub(super) callback: &'a mut dyn FnMut(&'static str) -> MyIden<'t>,
}

impl<'a, 't> TableRef<'a, 't> {
    pub fn get(&mut self, name: &'static str) -> MyIden<'t> {
        (self.callback)(name)
    }
}

pub trait Table<'t>: Copy {
    const NAME: &'static str;
    fn from_table(t: TableRef<'_, 't>) -> Self;
    fn into_row(&self) -> Vec<SimpleExpr>;
    // fn from_row(row: Vec<MyIden<'t>>) -> Self::Target<'t>;
}
