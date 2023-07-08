#![allow(unused)]

use sea_query::SimpleExpr;

use crate::orm::{
    query,
    row::Row,
    sub_query,
    value::{MyIden, Value},
    QueryOk, QueryRef, ReifyResRef, SubQuery,
};

#[derive(Clone, Copy)]
struct Instance<'t> {
    id: MyIden<'t>,
    timestamp: MyIden<'t>,
    problem: MyIden<'t>,
    seed: MyIden<'t>,
}

impl<'t> Row<'t> for Instance<'t> {
    fn into_row(&self) -> Vec<SimpleExpr> {
        vec![
            self.id.into_expr(),
            self.timestamp.into_expr(),
            self.problem.into_expr(),
            self.seed.into_expr(),
        ]
    }

    // fn from_row(row: Vec<MyIden<'t>>) -> Self {
    //     let mut row = row.into_iter();
    //     Self {
    //         id: row.next().unwrap(),
    //         timestamp: row.next().unwrap(),
    //         problem: row.next().unwrap(),
    //         seed: row.next().unwrap(),
    //     }
    // }
}

fn instances<'t>() -> SubQuery<Instance<'t>> {
    todo!()
}

#[derive(Clone, Copy)]
struct Execution<'t> {
    solution: MyIden<'t>,
    instance: MyIden<'t>,
}

impl<'t> Row<'t> for Execution<'t> {
    fn into_row(&self) -> Vec<SimpleExpr> {
        todo!()
    }

    // fn from_row(row: Vec<MyIden<'t>>) -> Self {
    //     todo!()
    // }
}

fn executions<'t>() -> SubQuery<Execution<'t>> {
    todo!()
}

#[derive(Clone, Copy)]
struct Solution<'t> {
    id: MyIden<'t>,
    file_hash: MyIden<'t>,
}

impl<'t> Row<'t> for Solution<'t> {
    fn into_row(&self) -> Vec<SimpleExpr> {
        todo!()
    }

    // fn from_row(row: Vec<MyIden<'t>>) -> Self {
    //     todo!()
    // }
}

fn solutions<'t>() -> SubQuery<Solution<'t>> {
    todo!()
}

#[derive(Clone, Copy)]
struct Problem<'t> {
    id: MyIden<'t>,
    file_hash: MyIden<'t>,
}

impl<'t> Row<'t> for Problem<'t> {
    fn into_row(&self) -> Vec<SimpleExpr> {
        todo!()
    }

    // fn from_row(row: Vec<MyIden<'t>>) -> Self {
    //     todo!()
    // }
}

fn problems<'t>() -> SubQuery<Problem<'t>> {
    todo!()
}

#[derive(Clone, Copy)]
struct Submission<'t> {
    solution: MyIden<'t>,
    problem: MyIden<'t>,
    timestamp: MyIden<'t>,
}

impl<'t> Row<'t> for Submission<'t> {
    fn into_row(&self) -> Vec<SimpleExpr> {
        todo!()
    }

    // fn from_row(row: Vec<MyIden<'t>>) -> Self {
    //     todo!()
    // }
}

fn submissions<'t>() -> SubQuery<Submission<'t>> {
    // let alias = iden();
    // SubQuery {
    //     select: Query::select()
    //         .from_as(Alias::new("submissions"), alias)
    //         .expr(Expr::table_asterisk(alias))
    //         .take(),
    //     row: Submission {
    //         solution: MyIden::from_expr(Expr::col((alias, Alias::new("solution")))),
    //         problem: todo!(),
    //         timestamp: todo!(),
    //     },
    // }
    todo!()
}

fn bench_instances<'a>() -> SubQuery<Instance<'a>> {
    sub_query(for<'t> |mut q: QueryRef<'t>| -> Instance<'t> {
        let instance = q.flat_map(instances());
        let mut same_problem = q.group_by(instance.problem);
        let is_new = same_problem.rank_desc(instance.timestamp).lt(5);
        q.filter(is_new);
        q.sort_by(instance.timestamp);
        instance
    })
}

fn sol_prob<'a>() -> SubQuery<(MyIden<'a>, MyIden<'a>)> {
    sub_query(for<'t> |mut q: QueryRef<'t>| -> (MyIden<'t>, MyIden<'t>) {
        let submission = q.flat_map(submissions());
        (submission.solution, submission.problem)
    })
}

fn sol_inst<'a>() -> SubQuery<(MyIden<'a>, MyIden<'a>)> {
    sub_query(for<'t> |mut q: QueryRef<'t>| -> (MyIden<'t>, MyIden<'t>) {
        let execution = q.flat_map(executions());
        (execution.solution, execution.instance)
    })
}

// last five problem-instances for each problem
// which have not been executed
fn bench_queue() -> QueryOk {
    query(for<'a> |mut q: QueryRef<'a>| -> ReifyResRef<'a> {
        // list of which solutions are submitted to which problems
        let submissions = sol_prob();

        // list of which solutions are executed on which instances
        let executions = sol_inst();

        // the relevant tables for our query
        let instance = q.flat_map(bench_instances());
        let solution = q.flat_map(solutions());
        let problem = q.flat_map(problems());

        q.filter(instance.problem.eq(problem.id));
        q.filter(submissions.contains((solution.id, problem.id)));
        q.filter(executions.contains((solution.id, instance.id)).not());

        q.reify(|mut r| QueuedExecution {
            instance_seed: 0,
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
