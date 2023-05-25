use sea_query::{
    Alias, Cond, Expr, Func, Iden, JoinType, Order, OrderedStatement, Query, SelectStatement,
    SimpleExpr, WindowStatement,
};
use std::{
    marker::PhantomData,
    ops::Not,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

pub fn iden() -> Rc<dyn Iden> {
    static IDEN_NUM: AtomicU64 = AtomicU64::new(0);
    let next = IDEN_NUM.fetch_add(1, Ordering::Relaxed);
    Rc::new(Alias::new(&next.to_string()))
}

#[derive(Clone)]
pub struct ValueRef<'t> {
    inner: SimpleExpr,
    _p: PhantomData<&'t mut &'t ()>,
}

impl<'t> ValueRef<'t> {
    pub fn from_expr(expr: impl Into<SimpleExpr>) -> Self {
        Self {
            inner: expr.into(),
            _p: PhantomData,
        }
    }

    pub fn lt(&self, arg: i32) -> Self {
        todo!()
    }

    pub fn eq(&self, other: Self) -> Self {
        todo!()
    }

    pub fn neg(&self) -> Self {
        let expr = Expr::expr(self.inner.clone()).mul(-1);
        Self::from_expr(expr)
    }
}

impl<'t> Not for ValueRef<'t> {
    type Output = Self;

    fn not(self) -> Self::Output {
        let expr = Expr::expr(self.inner).not();
        Self::from_expr(expr)
    }
}

pub struct GroupRef<'a, 't> {
    select: &'a mut SelectStatement,
    group: Vec<ValueRef<'t>>,
}

impl<'a, 't> GroupRef<'a, 't> {
    fn rank_internal(&mut self, val: impl Row<'t>, order: Order) -> ValueRef<'t> {
        let mut window = WindowStatement::new();
        for expr in val.into_row() {
            window.order_by_expr(expr.inner, order.clone());
        }
        let alias = iden();
        self.select
            .expr_window_as(Func::cust(Alias::new("ROW_NUMBER")), window, alias.clone());
        ValueRef::from_expr(Expr::col(alias))
    }

    pub fn rank_asc(&mut self, val: impl Row<'t>) -> ValueRef<'t> {
        self.rank_internal(val, Order::Asc)
    }

    pub fn rank_desc(&mut self, val: impl Row<'t>) -> ValueRef<'t> {
        self.rank_internal(val, Order::Desc)
    }
}

#[derive(Default)]
pub struct NewQuery<R> {
    select: SelectStatement,
    row: R,
}

pub struct QueryRef<'t> {
    select: &'t mut SelectStatement,
    _p: PhantomData<&'t mut &'t ()>,
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

pub trait Row<'t> {
    type Target<'a>: Row<'a>;

    fn into_row(&self) -> Vec<ValueRef<'t>>;
    fn from_row(row: Vec<ValueRef<'t>>) -> Self::Target<'t>;
}

impl<R> NewQuery<R>
where
    R: Row<'static>,
{
    pub fn new<F>(f: F) -> Self
    where
        for<'t> F: FnOnce(QueryRef<'t>) -> MapResRef<R>,
    {
        let mut select = Query::select();
        let q = QueryRef {
            select: &mut select,
            _p: PhantomData,
        };
        NewQuery {
            select,
            row: f(q).val,
        }
    }

    pub fn contains<'t>(self, val: R::Target<'t>) -> ValueRef<'t> {
        let val = val.into_row();
        let tuple = Expr::tuple(val.into_iter().map(|x| x.inner));
        ValueRef::from_expr(tuple.in_subquery(self.select))
    }
}

impl<'t> QueryRef<'t> {
    pub fn filter(&mut self, cond: ValueRef<'t>) {
        let alias = iden();
        *self.select = Query::select()
            .from_subquery(self.select.take(), alias.clone())
            .and_where(cond.inner)
            .expr(Expr::table_asterisk(alias))
            .take();
    }

    pub fn flat_map<O: Row<'static>>(&mut self, mut other: NewQuery<O>) -> O::Target<'t> {
        let (alias1, alias2) = (iden(), iden());
        *self.select = Query::select()
            .from_subquery(self.select.take(), alias1.clone())
            .join_subquery(
                JoinType::InnerJoin,
                other.select.take(),
                alias2.clone(),
                Cond::all(),
            )
            .expr(Expr::table_asterisk(alias1))
            .expr(Expr::table_asterisk(alias2))
            .take();
        other.row
    }

    // self is borrowed, because we need to mutate it to do group operations
    pub fn group_by<'a>(&'a mut self, group: impl Row<'t>) -> GroupRef<'a, 't> {
        GroupRef {
            select: &mut self.select,
            group: group.into_row(),
        }
    }

    pub fn sort_by(&mut self, order: impl Row<'t>) {
        for expr in order.into_row() {
            self.select.order_by_expr(expr.inner, Order::Asc);
        }
    }

    pub fn test(&mut self) -> ValueRef<'t> {
        todo!()
    }

    pub fn reify<T, F>(self, f: F) -> ReifyResRef<'t>
    where
        F: FnMut(ReifyRef<'t>) -> T,
    {
        todo!()
    }

    pub fn map<V: Row<'t>>(self, val: V) -> MapResRef<V::Target<'static>> {
        todo!()
    }
}

impl<'t> ReifyRef<'t> {
    pub fn get<V>(&mut self, v: &ValueRef<'t>) -> V {
        todo!()
    }
}

pub struct MapResRef<O> {
    val: O,
}

pub fn query<F>(f: F) -> QueryOk
where
    F: for<'t> FnOnce(QueryRef<'t>) -> ReifyResRef<'t>,
{
    let query = NewQuery::<Empty>::default();
    todo!()
    // query.ma
}

pub fn sub_query<O, F>(f: F) -> NewQuery<O>
where
    for<'t> F: FnOnce(QueryRef<'t>) -> MapResRef<O>,
    O: Row<'static>,
{
    NewQuery::new(f)
}

impl<'t> Row<'t> for ValueRef<'t> {
    type Target<'a> = ValueRef<'a>;

    fn into_row(&self) -> Vec<ValueRef<'t>> {
        vec![self.clone()]
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self {
        row[0].clone()
    }
}

impl<'x, 't, T: Row<'t>> Row<'t> for &'x T {
    type Target<'a> = T::Target<'a>;

    fn into_row(&self) -> Vec<ValueRef<'t>> {
        T::into_row(*self)
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self::Target<'t> {
        T::from_row(row)
    }
}

impl<'t, A: Row<'t>, B: Row<'t>> Row<'t> for (A, B) {
    type Target<'a> = (A::Target<'a>, B::Target<'a>);

    fn into_row(&self) -> Vec<ValueRef<'t>> {
        let mut res = self.0.into_row();
        res.extend(self.1.into_row());
        res
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self::Target<'t> {
        todo!()
    }
}

#[derive(Default)]
struct Empty;

impl<'t> Row<'t> for Empty {
    type Target<'a> = Empty;

    fn into_row(&self) -> Vec<ValueRef<'t>> {
        vec![]
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self {
        Empty
    }
}
