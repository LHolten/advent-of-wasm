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

pub struct MySelect(pub(super) Vec<Operation>);

pub struct MyDef {
    pub(super) table: Alias,
    pub(super) columns: Vec<(Alias, MyAlias)>,
}

pub(super) enum MyTable {
    Select(MySelect),
    Def(MyDef),
}

// push the query into a sub_query so that all columns are referenceable
pub fn push_down(select: &mut SelectStatement) {
    let inner = select.expr(Expr::asterisk()).take();
    *select = Query::select().from_subquery(inner, NullAlias).take();
}

impl MyDef {
    pub fn into_select(self) -> SelectStatement {
        let mut select = Query::select().from(self.table).take();
        for (col, alias) in self.columns {
            select.expr_as(Expr::col(col), alias);
        }
        select
    }
}

impl MySelect {
    pub fn into_select(self, then: Option<SimpleExpr>) -> SelectStatement {
        let mut select = Query::select();
        let mut stage = Stage::From;
        for op in self.0 {
            match op {
                Operation::From(table) => {
                    // we need to make sure that we are in the [From] stage
                    if stage > Stage::From {
                        push_down(&mut select);
                    }
                    let right = match table {
                        MyTable::Select(right) => right.into_select(None),
                        MyTable::Def(right) => right.into_select(),
                    };
                    select.from_subquery(right, NullAlias);
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
        if let Some(then) = then {
            // we need to push down windows
            if stage > Stage::Filter {
                push_down(&mut select)
            }
            select.expr(then);
        }
        select
    }
}
