keys:
- user_id
- solution_id
- problem_id
- instance_id

data:
- user_id -> user
- solution_id -> solution
- problem_id -> problem
- problem_id? + instance_id -> instance
- problem_id? + instance_id + solution_id -> execution
- problem_id + solution_id + user_id -> submission


queries:
- get all solutions by a user
- get all submissions of
- get all solutions for a problem
- get all scores for an instance (sorted by score?)
  

trees:
- user, problem, score, solution -> date
- problem, score, solution -> date // for each score system