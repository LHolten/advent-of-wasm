pub mod row;
pub mod value;

use phtm::InvariantOverLt;
use sea_query::{
    Alias, Cond, Expr, Func, Iden, JoinType, Order, OrderedStatement, OverStatement, Query,
    SelectStatement, SimpleExpr, WindowStatement,
};
use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use self::{
    row::Row,
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

impl<'a, 't, G: Row<'t>> GroupRef<'a, 't, G> {
    fn rank_internal(&mut self, val: impl Row<'t>, order: Order) -> impl Value<'t> {
        let mut window = WindowStatement::new();
        for expr in self.group.into_row() {
            window.add_partition_by(expr);
        }
        for expr in val.into_row() {
            window.order_by_expr(expr, order.clone());
        }
        let (alias1, alias2) = (MyAlias::new(), MyAlias::new());
        self.query.select = Query::select()
            .from_subquery(self.query.select.take(), alias1)
            .expr(Expr::table_asterisk(alias1))
            .expr_window_as(Func::cust(Alias::new("ROW_NUMBER")), window, alias2)
            .take();
        alias2.iden()
    }

    pub fn rank_asc(&mut self, val: impl Row<'t>) -> impl Value<'t> {
        self.rank_internal(val, Order::Asc)
    }

    pub fn rank_desc(&mut self, val: impl Row<'t>) -> impl Value<'t> {
        self.rank_internal(val, Order::Desc)
    }
}

pub struct SubQueryRes<R> {
    select: SelectStatement,
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

pub struct QueryRef<'t> {
    select: SelectStatement,
    _t: InvariantOverLt<'t>,
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
        let alias = MyAlias::new();
        let select = Query::select()
            .from_subquery(res.select, alias)
            .expr(Expr::tuple(res.row.into_row()))
            .take();
        Expr::tuple(self.val.into_row()).in_subquery(select)
    }
}

impl<'t> QueryRef<'t> {
    pub fn filter(&mut self, cond: impl Value<'t>) {
        let alias = MyAlias::new();
        self.select = Query::select()
            .from_subquery(self.select.take(), alias)
            .expr(Expr::table_asterisk(alias))
            .and_where(cond.into_expr())
            .take();
    }

    pub fn join<F>(&mut self, other: F) -> <F as SubQueryFunc<'t>>::Out
    where
        F: for<'a> SubQueryFunc<'a>,
    {
        let mut other_res = other.into_res();
        let (alias1, alias2) = (MyAlias::new(), MyAlias::new());
        self.select = Query::select()
            .from_subquery(self.select.take(), alias1)
            .from_subquery(other_res.select.take(), alias2)
            .expr(Expr::table_asterisk(alias1))
            .expr(Expr::table_asterisk(alias2))
            .take();
        other_res.row
    }

    // self is borrowed, because we need to mutate it to do group operations
    pub fn group_by<'a, G: Row<'t>>(&'a mut self, group: G) -> GroupRef<'a, 't, G> {
        GroupRef { query: self, group }
    }

    pub fn sort_by(&mut self, order: impl Row<'t>) {
        for expr in order.into_row() {
            self.select.order_by_expr(expr, Order::Asc);
        }
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
        select: Query::select(),
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
    type Out: Row<'t>;

    fn into_res(self) -> SubQueryRes<Self::Out> {
        let mut query = QueryRef {
            select: Query::select(),
            _t: PhantomData,
        };
        let row = (self)(&mut query);
        SubQueryRes {
            select: query.select,
            row,
        }
    }
}

impl<'t, O, F> SubQueryFunc<'t> for F
where
    F: FnOnce(&mut QueryRef<'t>) -> O,
    O: Row<'t>,
{
    type Out = O;
}
