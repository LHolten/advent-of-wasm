lifetimes are for column names
inner queries can use columns in outer queries
thus, inner queries have shorter lifetime
queries have unique, unchangable lifetime
values can be shortened in lifetime

`Value` is copy, it only contains column ids and operators.
`Row` is also copy, it contains a list of columns.
It is used for tuples and structs etc