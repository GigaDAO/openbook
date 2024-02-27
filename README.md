# Openbook v1 SDK

## Job Description
Translate OpenBook SDK from JS to Rust.
Source repository: https://github.com/openbook-dex/openbook-ts

Only the /packages/openbook directory needs to be translated. The rest of the packages can be imported as standard rust cargo dependencies. The swap package can be ignored (our use case only requires limit orders). Also, we do not require any of the functionality related to creating new markets, or using the crank.


## Acceptance Criteria
The Acceptance Criteria for this bounty is to implement the code snippet found in the README.md of the openbook package. I.e. execute the following functionalities:

- Load asks and bids for a target market.
- Place new limit orders
- View existing limit orders
- Cancel existing limit orders
- Retrieve fill history
- Settle funds

### Suggested Approach

This job has two primary components:
1) Serde for relevant state structures (e.g. critbit slab)
2) Building instructions (key metas and data inputs are in instruction.js)

Note that for component (1), you can just copy the structures directly from the contract source code, eg: https://github.com/openbook-dex/program/blob/master/dex/src/critbit.rs.

For component (2), you can get the data structs directly from the source, and the key metas from the instructions.js… this is just one approach. You can use whatever approach is more efficient for you. 

### Requirements
- `rustc -V == 1.74.1`

## A note to Bounty Hunters

This repo has two functions:
1) It will be used in our production code.
2) It will be used to determine which devs have enough skill to work with GigaDAO.

If you want to prove that you are a skilled Rust programmer, and thus receive paid bounty assignments, this is your 
opportunity. ***You do not have to implement the whole SDK.*** If you submit a PR for partially complete SDK, but it proves
your skill level, then I can assign you paid bounties. The minimum requirement all PR's is:
1) Must have working unit tests.
2) Must be clean and readable.

