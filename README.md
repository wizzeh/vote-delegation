# Vote Delegation Program

This program is a form of middleware that extends the functionality of Realms' governance delegate system. Instead of casting a vote for each user who has delegated their vote weight to the caller, they instead aggregate all that delegated weight into one voter weight record and cast one vote.

## Instructions
See documentation of individual instructions for more information.

- `set_precursor`: Sets up the middleware aspect of the program by designating the source of user voter weight.
- `create_voter_weight_record`: Creates an empty voter weight record. This record will be aggregated to by future transactions.
- `update_voter_weight_record`: Updates a voter weight record owned by the caller by aggregating the voter weight of delegating users.
- `revoke_vote`: Revokes voter weight which has been delegated by the caller using this program. This instruction can be called either before or after a vote has been cast, as long as the target still has voting open.
