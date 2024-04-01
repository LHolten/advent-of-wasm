// let average_fuel = (
//     from exec
//     group [exec.solution, exec.task] (
//         aggregate [
//             avg_fuel = average exec.fuel,
//         ]
//     )
// )

// # get solutions sorted by size for a person and task
// from submission
// join user [submission.user==user.id]
// filter user.token == "token"
// join task [submission.task==task.id]
// filter task.name == "task"
// join average_fuel [
//     submission.task==average_fuel.task,
//     submission.solution==average_fuel.solution,
// ]
// join solution [submission.solution==solution.id]
// sort solution.bytes_size
// select [solution.hash, solution.bytes_size, average_fuel.avg_fuel]

use crate::{
    orm::{
        table::Table,
        value::{Const, MyIden, Value},
        ContainsExt, QueryRef, ReifyRef,
    },
    queries::{Submission, User},
};

use super::{Execution, Instance, Problem};

#[derive(Clone, Copy)]
struct AverageFuel<'t> {
    solution: MyIden<'t>,
    problem: MyIden<'t>,
    fuel_avg: MyIden<'t>,
}

fn average_fuel<'t>(q: &mut QueryRef<'t>) -> AverageFuel<'t> {
    let exec = Execution::join(q);
    let inst = Instance::join(q);
    q.filter(exec.instance.eq(inst.id));

    let group = q.group_by((exec.solution, inst.problem));
    // group.average
    AverageFuel {
        solution: exec.solution,
        problem: inst.problem,
        fuel_avg: todo!(),
    }
}

impl User<'_> {
    fn by_github_id<'t>(q: &mut QueryRef<'t>, github_id: impl Value<'t>) -> User<'t> {
        let user = User::join(q);
        q.filter_eq(user.github_id, github_id);
        user
    }
}

struct SolutionStats {
    solution_hash: u64,
    bytes_size: u64,
    fuel_avg: u64,
}

fn get_solutions<'t>(
    q: &mut QueryRef<'t>,
    github_id: impl Value<'t>,
    problem_hash: u64,
) -> impl Fn(ReifyRef<'_, 't>) -> SolutionStats {
    let fuel = q.join(average_fuel);

    let cond = Submission::join.contains(Submission {
        solution: fuel.solution,
        problem: fuel.problem,
        timestamp: MyIden::null(),
        user: User::by_github_id(q, github_id).id,
    });
    q.filter(cond);

    let solution = q.join(fuel.solution);
    let problem = q.join(fuel.problem);
    q.filter_eq(problem.file_hash, problem_hash);

    |r| SolutionStats {
        solution_hash: r.get(solution.file_hash),
        bytes_size: r.get(solution.bytes_size),
        fuel_avg: r.get(fuel.fuel_avg),
    }
}
