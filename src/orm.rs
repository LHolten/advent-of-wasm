use sea_query::{
    Alias, ColumnRef, Cond, Condition, Expr, Func, Iden, JoinType, NullAlias, Order, OrderExpr,
    OrderedStatement, OverStatement, Query, SelectStatement, SimpleExpr, WindowStatement,
};
use std::{
    cell::Cell,
    marker::PhantomData,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

// enum Db<T> {
//     Real(T),
//     Iden(Rc<dyn Iden>),
//     Query(SelectStatement),
// }

struct MyQuery(SelectStatement);

impl MyQuery {
    pub fn filter<F>(self, f: F) -> Self
    where
        F: FnOnce(SimpleExpr) -> SimpleExpr,
    {
        let alias = iden();
        let filter = f(Expr::col(alias).into());

        let query = Query::select()
            .from_subquery(self.0, alias)
            .and_where(filter)
            .expr(Expr::asterisk())
            .take();
        Self(query)
    }

    pub fn map<R, F>(self, f: F) -> MyQuery
    where
        F: FnOnce(SimpleExpr) -> SimpleExpr,
    {
        let alias = iden();
        let map = f(Expr::col(alias).into());
        let query = Query::select()
            .from_subquery(self.0, alias)
            .expr(map)
            .take();
        Self(query)
    }
}

// impl GroupRef {
//     pub fn map<F>(mut self, f: F) -> MyQuery
//     where
//         F: FnOnce(GroupRef) -> SimpleExpr,
//     {
//         let res = f(self.group);
//         let query = self.query.add_group_by(self.group_by).expr(res).take();
//         MyQuery(query)
//     }

//     pub fn flat_map<F>(mut self, f: F) -> MyQuery
//     where
//         F: FnOnce(GroupRef) -> GroupRef,
//     {
//         let mut window = WindowStatement::new();

//         self.group = f(self.group);
//         if let Some(part) = self.group_by {
//             window.add_partition_by(part);
//         }
//         if let Some(ord) = self.group.sort_by {
//             window.add_order_by(ord);
//         }

//         let idx = iden();
//         self.query
//             .expr_window_as(Expr::cust("ROW_NUMBER()"), window, idx);

//         if let Some(n) = self.group.limit {
//             self.query.and_where(Expr::col(idx).lt(n));
//         }

//         MyQuery(self.query)
//     }
// }

pub fn iden() -> Rc<dyn Iden> {
    static IDEN_NUM: AtomicU64 = AtomicU64::new(0);
    let next = IDEN_NUM.fetch_add(1, Ordering::Relaxed);
    Rc::new(Alias::new(&next.to_string()))
}

impl<'a> ValueRef<'a> {
    pub fn is_same(self) -> WindowStatement {
        WindowStatement::new().add_partition_by(self.inner).take()
    }

    pub fn avg_of(self, window: WindowStatement) -> SimpleExpr {
        let (avg, subquery) = (iden(), iden());
        let expr = Expr::expr(self.inner).sum().div(Expr::asterisk().count());
        let select = Query::select()
            .from_subquery(
                self.select
                    .take()
                    .expr_window_as(expr, window, avg)
                    .expr(expr)
                    .take(),
                subquery,
            )
            .expr(Expr::table_asterisk(subquery));

        self.select.set(select.take());
        Expr::col(avg).into()
    }

    pub fn idx_of(self, mut window: WindowStatement) -> SimpleExpr {
        let (idx, subquery) = (iden(), iden());
        let expr = Expr::cust("ROW_NUMBER()");
        window.order_by_expr(self.inner, Order::Asc);

        let select = Query::select()
            .from_subquery(
                self.select
                    .take()
                    .expr_window_as(expr, window, idx)
                    .expr(expr)
                    .take(),
                subquery,
            )
            .expr(Expr::table_asterisk(subquery));

        self.select.set(select.take());
        Expr::col(idx).into()
    }
}

#[derive(Default)]
struct RowRef<'t> {
    exprs: Vec<ValueRef<'t>>,
}

impl<'t> RowRef<'t> {
    fn into_inner(self) -> SimpleExpr {
        todo!()
    }
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
}

struct GroupRef<'t> {
    select: &'t mut SelectStatement,
    group: RowRef<'t>,
}

impl<'t> GroupRef<'t> {
    pub fn rank(&mut self, val: RowRef<'t>) -> ValueRef<'t> {
        let mut window = WindowStatement::new();
        for expr in val.exprs {
            window.order_by_expr(expr.inner, Order::Asc);
        }
        let alias = iden();
        self.select
            .expr_window_as(Func::cust(Alias::new("ROW_NUMBER")), window, alias);
        ValueRef::from_expr(Expr::col(alias))
    }
}

#[derive(Default)]
struct NewQuery {
    select: SelectStatement,
    rows: RowRef<'static>,
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

impl NewQuery {
    pub fn map<F>(mut self, f: F) -> NewQuery
    where
        F: for<'a> FnOnce(QueryRef<'a>, RowRef<'a>) -> RowRef<'a>,
    {
        let q = QueryRef {
            select: &mut self.select,
            _p: PhantomData,
        };
        self.rows = f(q, self.rows);
        self
    }

    pub fn contains<'t>(self, val: RowRef<'t>) -> ValueRef<'t> {
        ValueRef::from_expr(Expr::expr(val.into_inner()).in_subquery(self.select))
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

    pub fn take(&mut self, n: u32) {
        todo!()
    }

    pub fn flat_map(&mut self, mut other: NewQuery) -> RowRef<'t> {
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
        other.rows
    }

    // self is borrowed, because we need to mutate it to do group operations
    pub fn group_by(&'t mut self, group: RowRef<'t>) -> GroupRef<'t> {
        GroupRef {
            select: &mut self.select,
            group,
        }
    }

    pub fn sort_by(&mut self, order: RowRef<'t>) {
        for expr in order.exprs {
            self.select.order_by_expr(expr.inner, Order::Asc);
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
    pub fn get<V>(&mut self, v: ValueRef<'t>) -> V {
        todo!()
    }
}

fn query<F>(f: F) -> NewQuery
where
    F: for<'t> FnOnce(QueryRef<'t>) -> ReifyResRef<'t>,
{
    let query = NewQuery::default();
    todo!()
    // query.ma
}

fn sub_query<F>(f: F) -> NewQuery
where
    F: for<'t> FnOnce(QueryRef<'t>) -> RowRef<'t>,
{
    let query = NewQuery::default();
    query.map(|q, r| f(q))
}

fn submissions() -> NewQuery {
    let alias = iden();
    NewQuery {
        select: Query::select()
            .from_as(Alias::new("submissions"), alias)
            .expr(Expr::table_asterisk(alias))
            .take(),
        rows: RowRef {
            exprs: vec![ValueRef::from_expr(Expr::col((
                alias,
                Alias::new("solution"),
            )))],
        },
    }
}

fn instances() -> NewQuery {
    todo!()
}

fn executions() -> NewQuery {
    todo!()
}

fn solutions() -> NewQuery {
    todo!()
}

fn problems() -> NewQuery {
    todo!()
}

fn bench_instances() -> NewQuery {
    sub_query(|q| {
        let instance = q.flat_map(instances());
        let same_problem = q.group_by(instance.problem);
        let is_new = same_problem.rank(-instance.timestamp) <= 5;
        q.filter(is_new);
        q.sort_by(instance.timestamp);
        instance
    })
}

// last five problem-instances for each problem
// which have not been executed
fn bench_queue() -> NewQuery {
    // list of which solutions are submitted to which problems
    let submissions = sub_query(|q| {
        let submission = q.flat_map(submissions());
        (submission.solution, submission.problem)
    });

    // list of which solutions are executed on which instances
    let executions = sub_query(|q| {
        let execution = q.flat_map(executions());
        (execution.solution, execution.instance)
    });

    query(|q| {
        // the relevant tables for our query
        let instance = q.flat_map(bench_instances());
        let solution = q.flat_map(solutions());
        let problem = q.flat_map(problems());

        q.filter(instance.problem == problem);
        q.filter(submissions.contains((solution, problem)));
        q.filter(!executions.contains((solution, instance)));

        q.map(|r| QueuedExecution {
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
