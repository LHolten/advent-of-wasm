use sea_query::{
    Alias, Cond, Expr, Func, Iden, JoinType, Order, OrderedStatement, Query, SelectStatement,
    SimpleExpr, WindowStatement,
};
use std::{
    marker::PhantomData,
    ops::{Neg, Not},
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

pub fn iden() -> Rc<dyn Iden> {
    static IDEN_NUM: AtomicU64 = AtomicU64::new(0);
    let next = IDEN_NUM.fetch_add(1, Ordering::Relaxed);
    Rc::new(Alias::new(&next.to_string()))
}

struct ValueRef<'t> {
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

    fn lt(&self, arg: i32) -> ValueRef {
        todo!()
    }

    fn eq(&self, problem: Problem) -> ValueRef {
        todo!()
    }
}

impl<'t> Neg for ValueRef<'t> {
    type Output = Self;

    fn neg(self) -> Self::Output {
        let expr = Expr::expr(self.inner).mul(-1);
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

struct GroupRef<'t> {
    select: &'t mut SelectStatement,
    group: Vec<ValueRef<'t>>,
}

impl<'t> GroupRef<'t> {
    pub fn rank(&mut self, val: impl Row<'t>) -> ValueRef<'t> {
        let mut window = WindowStatement::new();
        for expr in val.into_row() {
            window.order_by_expr(expr.inner, Order::Asc);
        }
        let alias = iden();
        self.select
            .expr_window_as(Func::cust(Alias::new("ROW_NUMBER")), window, alias);
        ValueRef::from_expr(Expr::col(alias))
    }
}

#[derive(Default)]
struct NewQuery<R: Table> {
    select: SelectStatement,
    row: R::Target<'static>,
}

struct QueryRef<'t> {
    select: &'t mut SelectStatement,
    _p: PhantomData<&'t mut &'t ()>,
}

struct ReifyRef<'t> {
    _p: PhantomData<&'t mut &'t ()>,
}

struct ReifyResRef<'t> {
    _p: PhantomData<&'t mut &'t ()>,
}

struct QueryOk {
    select: SelectStatement,
    reify: ReifyResRef<'static>,
}

impl<R> NewQuery<R>
where
    R: Table,
{
    pub fn map<O: Table, F>(mut self, f: F) -> NewQuery<O>
    where
        F: for<'a> FnOnce(QueryRef<'a>, R::Target<'a>) -> O::Target<'a>,
    {
        let q = QueryRef {
            select: &mut self.select,
            _p: PhantomData,
        };
        NewQuery {
            select: self.select,
            row: f(q, self.row),
        }
    }

    pub fn contains<'t>(self, val: impl Row<'t>) -> ValueRef<'t> {
        let val = val.into_row();
        let tuple = Expr::tuple(val.into_iter().map(|x| x.inner));
        ValueRef::from_expr(tuple.in_subquery(self.select))
    }
}

impl<'t> QueryRef<'t> {
    pub fn filter(&mut self, cond: ValueRef<'t>) {
        let alias = iden();
        *self.select = Query::select()
            .from_subquery(self.select.take(), alias)
            .and_where(cond.inner)
            .expr(Expr::table_asterisk(alias))
            .take();
    }

    pub fn flat_map<O: Table>(&mut self, mut other: NewQuery<O>) -> O::Target<'t> {
        let (alias1, alias2) = (iden(), iden());
        *self.select = Query::select()
            .from_subquery(self.select.take(), alias1)
            .join_subquery(
                JoinType::InnerJoin,
                other.select.take(),
                alias2,
                Cond::all(),
            )
            .expr(Expr::table_asterisk(alias1))
            .expr(Expr::table_asterisk(alias2))
            .take();
        other.row
    }

    // self is borrowed, because we need to mutate it to do group operations
    pub fn group_by(&'t mut self, group: impl Row<'t>) -> GroupRef<'t> {
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

    pub fn map<T, F>(self, f: F) -> ReifyResRef<'t>
    where
        F: FnMut(ReifyRef<'t>) -> T,
    {
        todo!()
    }
}

impl<'t> ReifyRef<'t> {
    pub fn get<V>(&mut self, v: ValueRef<'t>) -> V {
        todo!()
    }
}

fn query<F>(f: F) -> QueryOk
where
    F: for<'t> FnOnce(QueryRef<'t>) -> ReifyResRef<'t>,
{
    let query = NewQuery::<Empty>::default();
    todo!()
    // query.ma
}

fn sub_query<O: Table, F>(f: F) -> NewQuery<O>
where
    F: for<'t> FnOnce(QueryRef<'t>) -> O::Target<'t>,
{
    let query = NewQuery::<Empty>::default();
    query.map(|q, r| f(q))
}

pub trait Table {
    type Target<'t>: Row<'t>;
}

pub trait Row<'t>: Table<Target<'t> = Self> {
    fn into_row(self) -> Vec<ValueRef<'t>>;
    fn from_row(row: Vec<ValueRef<'t>>) -> Self;
}

impl<'t> Row<'t> for ValueRef<'t> {
    fn into_row(self) -> Vec<ValueRef<'t>> {
        vec![self]
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self {
        row[0]
    }
}

impl<'a> Table for ValueRef<'a> {
    type Target<'t> = ValueRef<'t>;
}

impl<'t, A: Row<'t>, B: Row<'t>> Row<'t> for (A, B) {
    fn into_row(self) -> Vec<ValueRef<'t>> {
        let mut res = self.0.into_row();
        res.extend(self.1.into_row());
        res
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self {
        todo!()
    }
}

impl<A: Table, B: Table> Table for (A, B) {
    type Target<'t> = (A::Target<'t>, B::Target<'t>);
}

#[derive(Default)]
struct Empty;
impl<'t> Row<'t> for Empty {
    fn into_row(self) -> Vec<ValueRef<'t>> {
        vec![]
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self {
        Empty
    }
}

impl Table for Empty {
    type Target<'t> = Empty;
}

struct Instance<'t> {
    timestamp: ValueRef<'t>,
    problem: ValueRef<'t>,
    seed: ValueRef<'t>,
}

impl<'t> Row<'t> for Instance<'t> {
    fn into_row(self) -> Vec<ValueRef<'t>> {
        vec![self.timestamp, self.problem, self.seed]
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self {
        let mut row = row.into_iter();
        Self {
            timestamp: row.next().unwrap(),
            problem: row.next().unwrap(),
            seed: row.next().unwrap(),
        }
    }
}

impl<'a> Table for Instance<'a> {
    type Target<'t> = Instance<'t>;
}

fn instances() -> NewQuery<Instance<'static>> {
    todo!()
}

struct Execution<'t> {
    solution: ValueRef<'t>,
    instance: ValueRef<'t>,
}

impl<'a> Table for Execution<'a> {
    type Target<'t> = Execution<'t>;
}

impl<'t> Row<'t> for Execution<'t> {
    fn into_row(self) -> Vec<ValueRef<'t>> {
        todo!()
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self {
        todo!()
    }
}

fn executions() -> NewQuery<Execution<'static>> {
    todo!()
}

struct Solution<'t> {
    file_hash: ValueRef<'t>,
}

impl<'a> Table for Solution<'a> {
    type Target<'t> = Solution<'t>;
}

impl<'t> Row<'t> for Solution<'t> {
    fn into_row(self) -> Vec<ValueRef<'t>> {
        todo!()
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self {
        todo!()
    }
}

fn solutions() -> NewQuery<Solution<'static>> {
    todo!()
}

struct Problem<'t> {
    file_hash: ValueRef<'t>,
}

impl<'a> Table for Problem<'a> {
    type Target<'t> = Problem<'t>;
}

impl<'t> Row<'t> for Problem<'t> {
    fn into_row(self) -> Vec<ValueRef<'t>> {
        todo!()
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self {
        todo!()
    }
}

fn problems() -> NewQuery<Problem<'static>> {
    todo!()
}

struct Submission<'t> {
    solution: ValueRef<'t>,
    problem: ValueRef<'t>,
    timestamp: ValueRef<'t>,
}

impl<'a> Table for Submission<'a> {
    type Target<'t> = Submission<'t>;
}

impl<'t> Row<'t> for Submission<'t> {
    fn into_row(self) -> Vec<ValueRef<'t>> {
        todo!()
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self {
        todo!()
    }
}

fn submissions() -> NewQuery<Submission<'static>> {
    let alias = iden();
    NewQuery {
        select: Query::select()
            .from_as(Alias::new("submissions"), alias)
            .expr(Expr::table_asterisk(alias))
            .take(),
        row: Submission {
            solution: ValueRef::from_expr(Expr::col((alias, Alias::new("solution")))),
            problem: todo!(),
            timestamp: todo!(),
        },
    }
}

fn bench_instances() -> NewQuery<Instance<'static>> {
    sub_query(|mut q| {
        let instance = q.flat_map(instances());
        let mut same_problem = q.group_by(instance.problem);
        let is_new = same_problem.rank(-instance.timestamp).lt(5);
        q.filter(is_new);
        q.sort_by(instance.timestamp);
        instance
    })
}

// last five problem-instances for each problem
// which have not been executed
fn bench_queue() -> QueryOk {
    // list of which solutions are submitted to which problems
    let submissions = sub_query(|mut q| {
        let submission = q.flat_map(submissions());
        (submission.solution, submission.problem)
    });

    // list of which solutions are executed on which instances
    let executions = sub_query(|mut q| {
        let execution = q.flat_map(executions());
        (execution.solution, execution.instance)
    });

    query(|mut q| {
        // the relevant tables for our query
        let instance = q.flat_map(bench_instances());
        let solution = q.flat_map(solutions());
        let problem = q.flat_map(problems());

        q.filter(instance.problem.eq(problem));
        q.filter(submissions.contains((solution, problem)));
        q.filter(!executions.contains((solution, instance)));

        q.map(|mut r| QueuedExecution {
            instance_seed: r.get(instance.seed),
            solution_hash: r.get(solution.file_hash),
            problem_hash: r.get(problem.file_hash),
        })
    })
}

struct QueuedExecution {
    instance_seed: u64,
    solution_hash: u64,
    problem_hash: u64,
}
