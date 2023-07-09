#![allow(unused)]

use std::{cell::OnceCell, ops::Deref};

use sea_query::SimpleExpr;

use crate::orm::{
    query,
    table::{Table, TableRef},
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

impl<'t> Table<'t> for Instance<'t> {
    const NAME: &'static str = "instance";

    fn from_table(mut t: TableRef<'_, 't>) -> Self {
        Self {
            id: t.get("id"),
            timestamp: t.get("timestamp"),
            problem: t.get("problem"),
            seed: t.get("seed"),
        }
    }
}

#[derive(Clone, Copy)]
struct Execution<'t> {
    solution: MyIden<'t>,
    instance: MyIden<'t>,
}

impl<'t> Table<'t> for Execution<'t> {
    const NAME: &'static str = "execution";

    fn from_table(mut t: TableRef<'_, 't>) -> Self {
        Self {
            solution: t.get("solution"),
            instance: t.get("instance"),
        }
    }
}

#[derive(Clone, Copy)]
struct Solution<'t> {
    id: MyIden<'t>,
    file_hash: MyIden<'t>,
}

impl<'t> Table<'t> for Solution<'t> {
    const NAME: &'static str = "solution";

    fn from_table(mut t: TableRef<'_, 't>) -> Self {
        Self {
            id: t.get("id"),
            file_hash: t.get("file_hash"),
        }
    }
}

#[derive(Clone, Copy)]
struct Problem<'t> {
    id: MyIden<'t>,
    file_hash: MyIden<'t>,
}

impl<'t> Table<'t> for Problem<'t> {
    const NAME: &'static str = "problem";

    fn from_table(mut t: TableRef<'_, 't>) -> Self {
        Self {
            id: t.get("id"),
            file_hash: t.get("file_hash"),
        }
    }
}

#[derive(Clone, Copy)]
struct Submission<'t> {
    solution: MyIden<'t>,
    problem: MyIden<'t>,
    timestamp: MyIden<'t>,
}

impl<'t> Table<'t> for Submission<'t> {
    const NAME: &'static str = "submission";

    fn from_table(mut t: TableRef<'_, 't>) -> Self {
        Self {
            solution: t.get("solution"),
            problem: t.get("problem"),
            timestamp: t.get("timestamp"),
        }
    }
}

fn bench_instances<'t>(q: &mut QueryRef<'t>) -> Instance<'t> {
    let instance: Instance = q.join_table();
    let mut same_problem = q.group_by(instance.problem);
    let is_new = same_problem.rank_desc(instance.timestamp).lt(5);
    q.filter(is_new);
    q.sort_by(instance.timestamp);
    instance
}

fn sol_prob<'t>(q: &mut QueryRef<'t>) -> (MyIden<'t>, MyIden<'t>) {
    let submission: Submission = q.join_table();
    (submission.solution, submission.problem)
}

fn sol_inst<'t>(q: &mut QueryRef<'t>) -> (MyIden<'t>, MyIden<'t>) {
    let execution: Execution = q.join_table();
    (execution.solution, execution.instance)
}

// last five problem-instances for each problem
// which have not been executed
fn bench_queue() -> QueryOk {
    query(for<'a> |mut q: QueryRef<'a>| -> ReifyResRef<'a> {
        // list of which solutions are submitted to which problems
        let submissions = SubQuery::new(sol_prob);

        // list of which solutions are executed on which instances
        let executions = SubQuery::new(sol_inst);

        // the relevant tables for our query
        let instance = q.join(bench_instances);
        let solution: Solution = q.join_table();
        let problem: Problem = q.join_table();

        q.filter(instance.problem.eq(problem.id));
        q.filter(submissions.contains((solution.id, problem.id)));
        q.filter(executions.contains((solution.id, instance.id)).not());

        q.reify(|mut r| QueuedExecution {
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

// pub fn boom<'t>(q: &mut QueryRef<'t>) {
//     // list of which solutions are submitted to which problems
//     let submissions = SubQuery::new(sol_prob);

//     let thing: Instance<'_> = q.flat_map(SubQuery::new(bench_instances));

//     let mut val: Option<Instance<'_>> = None;
//     let my_query = SubQuery::new(|q: &mut QueryRef<'t>| {
//         // q.filter(thing.problem);
//         val = Some(q.flat_map(SubQuery::new(bench_instances)));
//         todo!()
//     });

//     // q.filter(val.unwrap().problem);
// }

#[cfg(test)]
mod tests {
    use crate::orm::SubQueryFunc;

    use super::{bench_instances, sol_prob};

    #[test]
    fn print_sql() {
        (bench_instances).into_res().print()
    }
}
