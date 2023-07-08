pub mod row;
pub mod value;

use phtm::InvariantOverLt;
use sea_query::{
    Alias, Expr, Func, Iden, Order, OrderedStatement, Query, SelectStatement, SimpleExpr,
    WindowStatement,
};
use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use self::{
    row::{Empty, Row},
    value::{MyIden, Value},
};

pub fn iden<'t>() -> MyIden<'t> {
    todo!()
}

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
    fn rank_internal(&mut self, val: impl Row<'t>, order: Order) -> MyIden<'t> {
        let mut window = WindowStatement::new();
        for expr in val.into_row() {
            window.order_by_expr(expr, order.clone());
        }
        let alias = MyAlias::new();
        self.query
            .select
            .expr_window_as(Func::cust(Alias::new("ROW_NUMBER")), window, alias);
        alias.iden()
    }

    pub fn rank_asc(&mut self, val: impl Row<'t>) -> MyIden<'t> {
        self.rank_internal(val, Order::Asc)
    }

    pub fn rank_desc(&mut self, val: impl Row<'t>) -> MyIden<'t> {
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
        let val = self.val.into_row();
        let tuple = Expr::tuple(val);
        // tuple.in_subquery(
        //     Query::select()
        //         .expr(Expr::asterisk())
        //         .from(self.list.name)
        //         .take(),
        // )
        todo!()
    }
}

impl<'t> QueryRef<'t> {
    pub fn filter(&mut self, cond: impl Value<'t>) {
        let alias = iden();
        // *self.select = Query::select()
        //     .from_subquery(self.select.take(), alias.clone())
        //     .and_where(cond.into_expr(&mut self.token))
        //     .expr(Expr::table_asterisk(alias))
        //     .take();
        todo!()
    }

    pub fn join<F>(&mut self, mut other: F) -> <F as SubQueryFunc<'t>>::Out
    where
        F: for<'a> SubQueryFunc<'a>,
    {
        let (alias1, alias2) = (iden(), iden());
        // *self.select = Query::select()
        //     .from_subquery(self.select.take(), alias1.clone())
        //     .join_subquery(
        //         JoinType::InnerJoin,
        //         other.select.take(),
        //         alias2.clone(),
        //         Cond::all(),
        //     )
        //     .expr(Expr::table_asterisk(alias1))
        //     .expr(Expr::table_asterisk(alias2))
        //     .take();
        other.into_res().row
    }

    // // the query has a shorter, but unknown lifetime.
    // pub fn inline_query<F>(&mut self, f: F) -> T
    // where
    //     F: for<'a> FnOnce(&'a mut QueryRef<'t>) -> DynRow<'t>,
    // {
    //     todo!()
    // }

    // self is borrowed, because we need to mutate it to do group operations
    pub fn group_by<'a, G: Row<'t>>(&'a mut self, group: G) -> GroupRef<'a, 't, G> {
        GroupRef { query: self, group }
    }

    pub fn sort_by(&mut self, order: impl Row<'t>) {
        for expr in order.into_row() {
            self.select.order_by_expr(expr, Order::Asc);
        }
    }

    pub fn test(&mut self) -> MyIden<'t> {
        todo!()
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

pub fn query<F>(f: F) -> QueryOk
where
    F: for<'t> FnOnce(QueryRef<'t>) -> ReifyResRef<'t>,
{
    let query = SubQuery::<Empty>::default();
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
