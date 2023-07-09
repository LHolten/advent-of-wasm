lifetimes are for column names
inner queries can use columns in outer queries
thus, inner queries have shorter lifetime
queries have unique, unchangable lifetime
values can be shortened in lifetime

`Value` is copy, it only contains column ids and operators.
`Row` is also copy, it contains a list of columns.
It is used for tuples and structs etc


- from
    can use aliases in sqlite
- where
    can use aliases in sqlite
- group by
    can use aliases
- having
    can use aliases
- select
    can not use alias names, can in duckdb
- order by
    can use aliases
- limit
    can not use alias names 

# TODO
- automatic foreign key joins
- reference non-copy parameters from outside the query
- more expressions
- compile to CTEs for readability