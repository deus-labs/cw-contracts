use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Proposal not found")]
    ProposalNotFound {},

    #[error("Proposal period expired")]
    ProposalPeriodExpired {},

    #[error("Voting period expired")]
    VotingPeriodExpired {},

    #[error("Voting period not expired")]
    VotingPeriodNotExpired {},

    #[error("Wrong coin sent")]
    WrongCoinSent {},

    #[error("Wrong fund coin (expected: {expected}, got: {got})")]
    WrongFundCoin { expected: String, got: String },

    #[error("Address already voted project")]
    AddressAlreadyVotedProject {},

    #[error("CLR algorithm requires a budget constrain")]
    CLRConstrainRequired {},
}
