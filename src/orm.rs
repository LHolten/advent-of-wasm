mod ast;
pub mod row;
pub mod value;

use phtm::InvariantOverLt;
use sea_query::{
    Alias, Expr, Iden, Order, OrderedStatement, OverStatement, SelectStatement, SimpleExpr,
    WindowStatement,
};
use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::orm::row::TableRef;

use self::{
    ast::{MyDef, MySelect, MyTable, Operation},
    row::Table,
    value::{MyIden, Value},
};

#[derive(Clone, Copy)]
struct MyAlias(u64);
impl MyAlias {
    pub fn new() -> Self {
        static IDEN_NUM: AtomicU64 = AtomicU64::new(0);
        let next = IDEN_NUM.fetch_add(1, Ordering::Relaxed);
        Self(next)
    }

    pub fn iden<'t>(self) -> MyIden<'t> {
        todo!()
    }
}

impl Iden for MyAlias {
    fn unquoted(&self, s: &mut dyn std::fmt::Write) {
        write!(s, "{}", self.0).unwrap()
    }
}

pub struct GroupRef<'a, 't, G> {
    query: &'a mut QueryRef<'t>,
    group: G,
}

impl<'a, 't, G: Value<'t>> GroupRef<'a, 't, G> {
    fn rank_internal(&mut self, val: impl Value<'t>, order: Order) -> impl Value<'t> {
        let mut window = WindowStatement::new();
        window.add_partition_by(self.group.into_expr());
        window.order_by_expr(val.into_expr(), order);
        let alias = MyAlias::new();
        self.query
            .select
            .push(Operation::Window(val.into_expr(), window, alias));
        alias.iden()
    }

    pub fn rank_asc(&mut self, val: impl Value<'t>) -> impl Value<'t> {
        self.rank_internal(val, Order::Asc)
    }

    pub fn rank_desc(&mut self, val: impl Value<'t>) -> impl Value<'t> {
        self.rank_internal(val, Order::Desc)
    }
}

pub struct SubQueryRes<R> {
    ops: Vec<Operation>,
    row: R,
}

/// invariant is that `F` doesn't depend on anything else and nothing depends on it
/// this is checked by [SubQuery::new]
#[derive(Default, Clone, Copy)]
pub struct SubQuery<F>(F);

impl<F> SubQuery<F> {
    pub const fn new(func: F) -> Self
    where
        F: for<'a> SubQueryFunc<'a> + Copy,
    {
        SubQuery(func)
    }

    pub fn contains<'t>(self, val: F::Out) -> impl Value<'t>
    where
        F: SubQueryFunc<'t> + Copy,
    {
        Contains { func: self.0, val }
    }
}

pub struct ReifyRef<'t> {
    _t: InvariantOverLt<'t>,
}

pub struct ReifyResRef<'t> {
    _t: InvariantOverLt<'t>,
}

pub struct QueryOk {
    select: SelectStatement,
    reify: ReifyResRef<'static>,
}

#[derive(Clone, Copy)]
struct Contains<'t, F>
where
    F: SubQueryFunc<'t>,
{
    func: F,
    val: F::Out,
}

impl<'t, F: SubQueryFunc<'t> + Copy> Value<'t> for Contains<'t, F> {
    fn into_expr(self) -> SimpleExpr {
        let res = self.func.into_res();
        let select = MySelect(res.ops).into_select(Some(res.row.into_expr()));
        Expr::expr(self.val.into_expr()).in_subquery(select)
    }
}

pub struct QueryRef<'t> {
    select: Vec<Operation>,
    _t: InvariantOverLt<'t>,
}

impl<'t> QueryRef<'t> {
    pub fn filter(&mut self, cond: impl Value<'t>) {
        self.select.push(Operation::Filter(cond.into_expr()));
    }

    pub fn join<F>(&mut self, other: F) -> <F as SubQueryFunc<'t>>::Out
    where
        F: for<'a> SubQueryFunc<'a>,
    {
        let other_res = other.into_res();
        self.select
            .push(Operation::From(MyTable::Select(MySelect(other_res.ops))));
        other_res.row
    }

    pub fn join_table<T: Table<'t>>(&mut self) -> T {
        let mut columns = Vec::new();
        let res = T::from_table(TableRef {
            callback: &|name| {
                let alias = MyAlias::new();
                columns.push((Alias::new(name), alias));
                alias.iden()
            },
        });
        self.select.push(Operation::From(MyTable::Def(MyDef {
            table: Alias::new(T::NAME),
            columns,
        })));
        res
    }

    // self is borrowed, because we need to mutate it to do group operations
    pub fn group_by<'a, G: Value<'t>>(&'a mut self, group: G) -> GroupRef<'a, 't, G> {
        GroupRef { query: self, group }
    }

    pub fn sort_by(&mut self, order: impl Value<'t>) {
        self.select
            .push(Operation::Order(order.into_expr(), Order::Asc));
    }

    pub fn reify<T, F>(self, f: F) -> ReifyResRef<'t>
    where
        F: FnMut(ReifyRef<'t>) -> T,
    {
        todo!()
    }
}

impl<'t> ReifyRef<'t> {
    pub fn get<V>(&mut self, v: impl Value<'t>) -> V {
        todo!()
    }
}

pub fn query<F>(func: F) -> QueryOk
where
    F: for<'t> FnOnce(QueryRef<'t>) -> ReifyResRef<'t>,
{
    let query = QueryRef {
        select: Vec::new(),
        _t: PhantomData,
    };
    let res = (func)(query);
    QueryOk {
        select: todo!(),
        reify: todo!(),
    };
    todo!()
    // query.ma
}

pub trait SubQueryFunc<'t>: Sized
where
    Self: FnOnce(&mut QueryRef<'t>) -> Self::Out,
{
    type Out: Value<'t>;

    fn into_res(self) -> SubQueryRes<Self::Out> {
        let mut query = QueryRef {
            select: Vec::new(),
            _t: PhantomData,
        };
        let row = (self)(&mut query);
        SubQueryRes {
            ops: query.select,
            row,
        }
    }
}

impl<'t, O, F> SubQueryFunc<'t> for F
where
    F: FnOnce(&mut QueryRef<'t>) -> O,
    O: Value<'t>,
{
    type Out = O;
}
