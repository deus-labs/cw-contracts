# CosmWasm Quadratic Funding

A CosmWasm smart contract, written in Rust, that instantiates a “Quadratic Funding Round”, and has the following functionality:

- Coins sent with contract instantiation goes into the common funding pool, to be reallocated based on the Quadratic Funding formula.
- Contract parameterized by a list of whitelisted addresses who can make proposals, and a list of whitelisted addresses who can vote on proposals. Empty list means permissionless proposals/voting.
- Proposers can make text based proposals via contract function calls, and set the address that funds should be sent to.
- Proposal periods / voting periods are either defined in advance by contract parameters, or are explicitly triggered via function calls from contract creator / admin.
    If periods are triggered via function calls, minimum proposal periods / voting periods should be set upon contract instantiation.
- Voters vote on proposals by sending coins to the contract in a function call referencing a proposal.
- Once voting period ends, contract creator / admin triggers the distribution of funds to proposals according to a quadratic funding formula.
- A web based user interface (using CosmJS) with the following functionality:
- Allows instantiation of a new contract / creation of new proposal round
- Enables sending of proposals, and voting on proposals
- Enables viewing all proposals and votes for a given contract / Quadratic Funding Round
- Video demo showcasing functionality of your Quadratic Funding dApp!

## Bonus Points

- [ ] Support for alternative funding formulas (besides the standard quadratic funding formula)
- [ ] Support for structured proposal metadata
- [ ] Support for multiple funding rounds per contract
- [ ] Variable proposal periods / voting periods
- [ ] Support for more fine grained queries like “get proposal text/metadata by proposal ID”
- [ ] Regen Network / OpenTEAM logos & branding represented in the UI
- [ ] Deploy your contract to the CosmWasm coral testnet, and share a working link to your dApp

First iteration will only support single type of native coin.

## Messages

```rust
pub struct InitMsg {
    pub admin: HumanAddr,
    pub leftover_addr: HumanAddr,
    pub create_proposal_whitelist: Option<Vec<HumanAddr>>,
    pub vote_proposal_whitelist: Option<Vec<HumanAddr>>,
    pub voting_period: Expiration,
    pub proposal_period: Expiration,
    pub budget_denom: String,
    pub algorithm: QuadraticFundingAlgorithm,
}

pub enum HandleMsg {
    CreateProposal {
        title: String,
        description: String,
        metadata: Option<Binary>,
        fund_address: HumanAddr,
    },
    VoteProposal {
        proposal_id: u64,
    },
    TriggerDistribution {},
}
```

### State

```rust
pub struct Config {
    // set admin as single address, multisig or contract sig could be used
    pub admin: CanonicalAddr,
    // leftover coins from distribution sent to this address
    pub leftover_addr: CanonicalAddr,
    pub create_proposal_whitelist: Option<Vec<CanonicalAddr>>,
    pub vote_proposal_whitelist: Option<Vec<CanonicalAddr>>,
    pub voting_period: Expiration,
    pub proposal_period: Expiration,
    pub budget: Coin,
    pub algorithm: QuadraticFundingAlgorithm,
}

pub struct Proposal {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub metadata: Option<Binary>,
    pub fund_address: CanonicalAddr,
    pub collected_funds: Uint128,
}
pub struct Vote {
    pub proposal_id: u64,
    pub voter: CanonicalAddr,
    pub fund: Coin,
}
```

### Queries

```rust
pub enum QueryMsg {
    ProposalByID { id: u64 },
    AllProposals {},
}
```

## Iteration 2

Support CW20
