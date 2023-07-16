use sea_query::Alias;

use super::{
    ast::{MyDef, MyTable, Operation},
    value::MyIden,
    MyAlias, QueryRef,
};

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
    // fn into_row(&self) -> Vec<SimpleExpr>;
    // fn from_row(row: Vec<MyIden<'t>>) -> Self::Target<'t>;
}

impl<'t> QueryRef<'t> {
    pub fn join_table<T: Table<'t>>(&mut self) -> T {
        let mut columns = Vec::new();
        let res = T::from_table(TableRef {
            callback: &mut |name| {
                let alias = MyAlias::new();
                columns.push((Alias::new(name), alias));
                alias.iden()
            },
        });
        self.ops.push(Operation::From(MyTable::Def(MyDef {
            table: Alias::new(T::NAME),
            columns,
        })));
        res
    }
}
