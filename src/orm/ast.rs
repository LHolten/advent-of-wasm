use sea_query::{
    Alias, Expr, NullAlias, Order, Query, SelectStatement, SimpleExpr, WindowStatement,
};

use super::MyAlias;

// invariant: columns need to be joined before they are used
pub(super) enum Operation {
    // the new column names must all be MyAlias
    From(MyTable),
    // can make use of stuff in [From]
    Filter(SimpleExpr),
    // can only make use of stuff in [From]
    Window(SimpleExpr, WindowStatement, MyAlias),
    // can make use of stuff in [From] and [Select],
    Order(SimpleExpr, Order),
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum Stage {
    From,
    Filter,
    Order,
}

pub(super) enum MyTable {
    Select(Vec<Operation>),
    Def {
        table: Alias,
        columns: Vec<(Alias, MyAlias)>,
    },
}

// push the query into a sub_query so that all columns are referenceable
pub fn push_down(select: &mut SelectStatement) {
    let inner = select.expr(Expr::asterisk()).take();
    *select = Query::select().from_subquery(inner, NullAlias).take();
}

impl MyTable {
    pub fn into_select(self) -> SelectStatement {
        let operations = match self {
            MyTable::Select(ops) => ops,
            MyTable::Def { table, columns } => {
                let mut select = Query::select().from(table).take();
                for (col, alias) in columns {
                    select.expr_as(Expr::col(col), alias);
                }
                return select;
            }
        };

        let mut select = Query::select();
        let mut stage = Stage::From;
        for op in operations {
            match op {
                Operation::From(table) => {
                    // we need to make sure that we are in the [From] stage
                    if stage > Stage::From {
                        push_down(&mut select);
                    }
                    select.from_subquery(table.into_select(), NullAlias);
                    stage = Stage::From;
                }
                Operation::Filter(expr) => {
                    if stage > Stage::Filter {
                        push_down(&mut select);
                    }
                    select.and_where(expr);
                    stage = Stage::Filter;
                }
                Operation::Window(expr, window, alias) => {
                    // we are not allowed to use one window result in another
                    // that is why we need to push down other window operations
                    if stage > Stage::Filter {
                        push_down(&mut select);
                    }
                    select.expr_window_as(expr, window, alias);
                    stage = Stage::Order
                }
                Operation::Order(expr, order) => {
                    select.order_by_expr(expr, order);
                    stage = Stage::Order
                }
            }
        }
        select.expr(Expr::asterisk()).take()
    }
}
