mod ast;
pub mod table;
pub mod value;

use phtm::InvariantOverLt;
use rusqlite::{types::FromSql, Connection, Row};
use sea_query::{
    Expr, Iden, Order, OrderedStatement, OverStatement, SimpleExpr, SqliteQueryBuilder,
    WindowStatement,
};
use sea_query_rusqlite::RusqliteBinder;
use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use self::{
    ast::{MySelect, MyTable, Operation},
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
        MyIden {
            name: self,
            _t: PhantomData,
        }
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
            .ops
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

#[cfg(test)]
impl<R> SubQueryRes<R> {
    pub fn print(self) {
        let select = MySelect(self.ops).into_select(None);
        let res = select.build(sea_query::SqliteQueryBuilder);
        println!("{}", res.0);
    }
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
        F::Out: Value<'t>,
    {
        Contains { func: self.0, val }
    }
}

#[derive(Clone, Copy)]
struct Contains<'t, F>
where
    F: SubQueryFunc<'t>,
{
    func: F,
    val: F::Out,
}

impl<'t, F: SubQueryFunc<'t> + Copy> Value<'t> for Contains<'t, F>
where
    F::Out: Value<'t>,
{
    fn into_expr(self) -> SimpleExpr {
        let res = self.func.into_res();
        let select = MySelect(res.ops).into_select(Some(res.row.into_expr()));
        Expr::expr(self.val.into_expr()).in_subquery(select)
    }
}

pub struct QueryRef<'t> {
    ops: Vec<Operation>,
    _t: InvariantOverLt<'t>,
}

impl<'t> QueryRef<'t> {
    pub fn filter(&mut self, cond: impl Value<'t>) {
        self.ops.push(Operation::Filter(cond.into_expr()));
    }

    pub fn join<F>(&mut self, other: F) -> <F as SubQueryFunc<'t>>::Out
    where
        F: for<'a> SubQueryFunc<'a>,
    {
        let other_res = other.into_res();
        self.ops
            .push(Operation::From(MyTable::Select(MySelect(other_res.ops))));
        other_res.row
    }

    // self is borrowed, because we need to mutate it to do group operations
    pub fn group_by<'a, G: Value<'t>>(&'a mut self, group: G) -> GroupRef<'a, 't, G> {
        GroupRef { query: self, group }
    }

    pub fn sort_by(&mut self, order: impl Value<'t>) {
        self.ops
            .push(Operation::Order(order.into_expr(), Order::Asc));
    }
}

pub trait SubQueryFunc<'t>: Sized
where
    Self: FnOnce(&mut QueryRef<'t>) -> Self::Out,
{
    type Out: Copy + 't;

    fn into_res(self) -> SubQueryRes<Self::Out> {
        let mut query = QueryRef {
            ops: Vec::new(),
            _t: PhantomData,
        };
        let row = (self)(&mut query);
        SubQueryRes {
            ops: query.ops,
            row,
        }
    }
}

impl<'t, O, F> SubQueryFunc<'t> for F
where
    F: FnOnce(&mut QueryRef<'t>) -> O,
    O: Copy + 't,
{
    type Out = O;
}

pub trait QueyFunc<'t>
where
    Self: FnOnce(&mut QueryRef<'t>) -> Self::Out,
{
    type Out: Fn(ReifyRef<'_, 't>) -> Self::Final;
    type Final;
}

impl<'t, F, O, T> QueyFunc<'t> for F
where
    Self: FnOnce(&mut QueryRef<'t>) -> O,
    O: Fn(ReifyRef<'_, 't>) -> T,
{
    type Out = O;
    type Final = T;
}

pub struct ReifyRef<'a, 't> {
    row: &'a Row<'a>,
    _t: InvariantOverLt<'t>,
}

impl<'a, 't> ReifyRef<'a, 't> {
    pub fn get<V: FromSql>(&mut self, v: MyIden<'t>) -> V {
        let mut name = String::new();
        v.name.unquoted(&mut name);
        self.row.get_unwrap(&*name)
    }
}

pub fn query<F, T>(func: F) -> Vec<T>
where
    F: for<'t> QueyFunc<'t, Final = T>,
{
    let mut query = QueryRef {
        ops: Vec::new(),
        _t: PhantomData,
    };
    let res = (func)(&mut query);

    let select = MySelect(query.ops).into_select(None);
    let (query, params) = select.build_rusqlite(SqliteQueryBuilder);

    let conn = Connection::open("test.db").unwrap();
    let mut stmt = conn.prepare(&query).unwrap();
    stmt.query_map(&*params.as_params(), |row| {
        let reify = ReifyRef {
            _t: PhantomData,
            row,
        };
        Ok((res)(reify))
    })
    .unwrap()
    .collect::<Result<_, _>>()
    .unwrap()
}
