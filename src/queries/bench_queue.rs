use crate::orm::{
    query,
    table::Table,
    value::{MyIden, Value},
    ContainsExt, QueryRef, ReifyRef,
};

use super::{Execution, Instance, Problem, Solution, Submission};

fn bench_instances<'t>(q: &mut QueryRef<'t>) -> Instance<'t> {
    let instance = Instance::join(q);
    let mut same_problem = q.group_by(instance.problem);
    let is_new = same_problem.rank_desc(instance.timestamp).lt(5);
    q.filter(is_new);
    q.sort_by(instance.timestamp);
    instance
}

// list of which solutions are submitted to which problems
fn sol_prob<'t>(q: &mut QueryRef<'t>) -> (MyIden<'t>, MyIden<'t>) {
    let submission = Submission::join(q);
    (submission.solution, submission.problem)
}

// list of which solutions are executed on which instances
fn sol_inst<'t>(q: &mut QueryRef<'t>) -> (MyIden<'t>, MyIden<'t>) {
    let execution = Execution::join(q);
    (execution.solution, execution.instance)
}

fn bench_inner<'t>(q: &mut QueryRef<'t>) -> impl Fn(ReifyRef<'_, 't>) -> QueuedExecution {
    // the relevant tables for our query
    let instance = q.join(bench_instances);
    let solution = Solution::join(q);
    let problem = Problem::join(q);

    q.filter(instance.problem.eq(problem.id));
    q.filter(sol_prob.contains((solution.id, problem.id)));
    q.filter(sol_inst.contains((solution.id, instance.id)).not());

    move |mut r: ReifyRef| QueuedExecution {
        instance_seed: r.get(instance.seed),
        solution_hash: r.get(solution.file_hash),
        problem_hash: r.get(problem.file_hash),
    }
}

// last five problem-instances for each problem
// which have not been executed
fn bench_queue() -> Vec<QueuedExecution> {
    query(bench_inner)
}

struct QueuedExecution {
    instance_seed: u64,
    solution_hash: u64,
    problem_hash: u64,
}

#[cfg(test)]
mod tests {
    use crate::orm::SubQueryFunc;

    use super::{bench_instances, sol_prob};

    #[test]
    fn print_sql() {
        (bench_instances).into_res().print()
    }
}
