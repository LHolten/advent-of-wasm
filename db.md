solution
- hash
- size

task -* solution
user -* submission
solution -* submission
task -* instance
instance -* score
solution -* score

solution
- task
submission
- user
- solution
instance
- task
score
- instance
- solution

total solution score:
sum (filter score by solution)

list users for solution
filter submission by solution