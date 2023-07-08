pub mod row;
pub mod value;

use phtm::{CovariantOverLt, InvariantOverLt};
use sea_query::{
    Alias, Expr, Func, Iden, Order, OrderedStatement, SelectStatement, SimpleExpr, WindowStatement,
};
use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU64, Ordering},
};

use self::{
    row::{DynRow, Empty, Row},
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

#[derive(Default)]
pub struct SubQuery<R> {
    select: SelectStatement,
    row: R,
}

pub struct QueryRef<'t> {
    select: &'t mut SelectStatement,
    token: InvariantOverLt<'t>,
}

pub struct Table<'t> {
    select: SelectStatement,
    _t: CovariantOverLt<'t>,
}

pub struct ReifyRef<'t> {
    _p: PhantomData<&'t mut &'t ()>,
}

pub struct ReifyResRef<'t> {
    _p: PhantomData<&'t mut &'t ()>,
}

pub struct QueryOk {
    select: SelectStatement,
    reify: ReifyResRef<'static>,
}

#[derive(Clone, Copy)]
struct Contains<'a, R> {
    list: &'a SubQuery<R>,
    val: R,
}

impl<'a, 't, R: Row<'t>> Value<'t> for Contains<'a, R> {
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

impl<'t, R: Row<'t>> SubQuery<R> {
    pub fn new<F>(f: F) -> Self
    where
        F: FnOnce(QueryRef<'t>) -> R,
    {
        // GhostToken::new(|token| {
        //     let mut select = Query::select();
        //     let q = QueryRef {
        //         select: &mut select,
        //         token,
        //     };
        //     let row = f(q);
        //     SubQuery { select, row }
        // })
        todo!()
    }

    pub fn contains(&self, val: R) -> impl Value<'t> + '_ {
        Contains { list: self, val }
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

    pub fn flat_map<O: Row<'t>>(&mut self, mut other: SubQuery<O>) -> O {
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
        other.row
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

pub fn sub_query<'t, F>(f: F) -> SubQuery<<F as SubQueryFunc<'t>>::Out>
where
    F: for<'a> SubQueryFunc<'a>,
{
    todo!()
}

pub trait SubQueryFunc<'t>
where
    Self: FnOnce(QueryRef<'t>) -> Self::Out,
{
    type Out: Row<'t>;
}

impl<'t, O, F> SubQueryFunc<'t> for F
where
    F: FnOnce(QueryRef<'t>) -> O,
    O: Row<'t>,
{
    type Out = O;
}
