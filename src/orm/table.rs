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

pub trait Table: Copy {
    const NAME: &'static str;

    type Out<'t>: 't;

    fn from_table<'t>(t: TableRef<'_, 't>) -> Self::Out<'t>;

    fn join<'t>(q: &mut QueryRef<'t>) -> Self::Out<'t> {
        let mut columns = Vec::new();
        let res = Self::from_table(TableRef {
            callback: &mut |name| {
                let alias = MyAlias::new();
                columns.push((Alias::new(name), alias));
                alias.iden()
            },
        });
        q.ops.push(Operation::From(MyTable::Def(MyDef {
            table: Alias::new(Self::NAME),
            columns,
        })));
        res
    }
}
