use std::ops::Deref;

use sea_query::{Alias, Expr, Query};

use crate::orm::{iden, query, sub_query, NewQuery, QueryOk, Row, ValueRef};

struct Instance<'t> {
    id: ValueRef<'t>,
    timestamp: ValueRef<'t>,
    problem: ValueRef<'t>,
    seed: ValueRef<'t>,
}

impl<'t> Row<'t> for Instance<'t> {
    type Target<'a> = Instance<'a>;

    fn into_row(&self) -> Vec<ValueRef<'t>> {
        vec![
            self.id.clone(),
            self.timestamp.clone(),
            self.problem.clone(),
            self.seed.clone(),
        ]
    }

    fn from_row(row: Vec<ValueRef<'t>>) -> Self {
        let mut row = row.into_iter();
        Self {
            id: row.next().unwrap(),
            timestamp: row.next().unwrap(),
            problem: row.next().unwrap(),
            seed: row.next().unwrap(),
        }
    }
}

fn instances() -> NewQuery<Instance<'static>> {
    todo!()
}

struct Execution<'t> {
    solution: ValueRef<'t>,
    instance: ValueRef<'t>,
}

impl<'t> Row<'t> for Execution<'t> {
    type Target<'a> = Execution<'a>;

    fn into_row(&self) -> Vec<ValueRef<'t>> {
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
    id: ValueRef<'t>,
    file_hash: ValueRef<'t>,
}

impl<'t> Row<'t> for Solution<'t> {
    type Target<'a> = Solution<'a>;

    fn into_row(&self) -> Vec<ValueRef<'t>> {
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
    id: ValueRef<'t>,
    file_hash: ValueRef<'t>,
}

impl<'t> Row<'t> for Problem<'t> {
    type Target<'a> = Problem<'a>;

    fn into_row(&self) -> Vec<ValueRef<'t>> {
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

impl<'t> Row<'t> for Submission<'t> {
    type Target<'a> = Submission<'a>;

    fn into_row(&self) -> Vec<ValueRef<'t>> {
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
            .from_as(Alias::new("submissions"), alias.clone())
            .expr(Expr::table_asterisk(alias.clone()))
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
        let mut same_problem = q.group_by(&instance.problem);
        let is_new = same_problem.rank_asc(instance.timestamp.neg()).lt(5);
        q.filter(is_new);
        q.sort_by(&instance.timestamp);
        q.map(instance)
    })
}

// last five problem-instances for each problem
// which have not been executed
fn bench_queue() -> QueryOk {
    // list of which solutions are submitted to which problems
    let submissions = sub_query(|mut q| {
        let submission = q.flat_map(submissions());
        q.map((submission.solution, submission.problem))
    });

    // list of which solutions are executed on which instances
    let executions = sub_query(|mut q| {
        let execution = q.flat_map(executions());
        q.map((execution.solution, execution.instance))
    });

    query(|mut q| {
        // the relevant tables for our query
        let instance = q.flat_map(bench_instances());
        let solution = q.flat_map(solutions());
        let problem = q.flat_map(problems());

        // q.filter(instance.problem.eq(problem));
        q.filter(submissions.contains((solution.id.clone(), problem.id)));
        q.filter(!executions.contains((solution.id, instance.id)));

        q.reify(|mut r| QueuedExecution {
            instance_seed: r.get(&instance.seed),
            solution_hash: r.get(&solution.file_hash),
            problem_hash: r.get(&problem.file_hash),
        })
    })
}

struct QueuedExecution {
    instance_seed: u64,
    solution_hash: u64,
    problem_hash: u64,
}
