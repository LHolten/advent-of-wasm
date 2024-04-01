#![allow(unused)]
mod bench_queue;
mod query;

use std::{cell::OnceCell, ops::Deref};

use sea_query::SimpleExpr;

use crate::orm::{
    query,
    table::{Table, TableRef},
    value::{MyIden, Value},
    ContainsExt, QueryRef, ReifyRef,
};

#[derive(Clone, Copy)]
struct Instance<'t> {
    id: MyIden<'t>,
    timestamp: MyIden<'t>,
    problem: MyIden<'t>,
    seed: MyIden<'t>,
}

impl Table for Instance<'_> {
    const NAME: &'static str = "instance";

    type Out<'t> = Instance<'t>;

    fn from_table<'t>(mut t: TableRef<'_, 't>) -> Self::Out<'t> {
        Self::Out {
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

impl Table for Execution<'_> {
    const NAME: &'static str = "execution";

    type Out<'t> = Execution<'t>;

    fn from_table<'t>(mut t: TableRef<'_, 't>) -> Self::Out<'t> {
        Self::Out {
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

impl Table for Solution<'_> {
    const NAME: &'static str = "solution";

    type Out<'t> = Solution<'t>;

    fn from_table<'t>(mut t: TableRef<'_, 't>) -> Self::Out<'t> {
        Self::Out {
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

impl Table for Problem<'_> {
    const NAME: &'static str = "problem";

    type Out<'t> = Problem<'t>;

    fn from_table<'t>(mut t: TableRef<'_, 't>) -> Self::Out<'t> {
        Self::Out {
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
    user: MyIden<'t>,
}

impl Table for Submission<'_> {
    const NAME: &'static str = "submission";

    type Out<'t> = Submission<'t>;

    fn from_table<'t>(mut t: TableRef<'_, 't>) -> Self::Out<'t> {
        Self::Out {
            solution: t.get("solution"),
            problem: t.get("problem"),
            timestamp: t.get("timestamp"),
            user: t.get("user"),
        }
    }
}

#[derive(Clone, Copy)]
struct User<'t> {
    id: MyIden<'t>,
    timestamp: MyIden<'t>,
    github_id: MyIden<'t>,
}

impl Table for User<'_> {
    const NAME: &'static str = "user";

    type Out<'t> = User<'t>;

    fn from_table<'t>(mut t: TableRef<'_, 't>) -> Self::Out<'t> {
        Self::Out {
            id: t.get("id"),
            timestamp: t.get("timestamp"),
            github_id: t.get("github_id"),
        }
    }
}

// pub fn boom<'t>(q: &mut QueryRef<'t>) {
//     let solution: Solution = q.join_table();

//     let f = |g: &mut QueryRef<'t>| solution.id;

//     f.contains(solution.id);

//     todo!()

//     // let thing: Instance<'_> = q.flat_map(SubQuery::new(bench_instances));

//     // let mut val: Option<Instance<'_>> = None;
//     // let my_query = SubQuery::new(|q: &mut QueryRef<'t>| {
//     //     // q.filter(thing.problem);
//     //     val = Some(q.flat_map(SubQuery::new(bench_instances)));
//     //     todo!()
//     // });

//     // q.filter(val.unwrap().problem);
// }
