use std::collections::HashMap;

use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};

use crate::msg::{CreatePollInput, ExecuteMsg, HasVotedResponse, InstantiateMsg, PollCountResponse, PollResponse, QueryMsg, ResultsResponse, VoteCountResponse, VoteInput};
use crate::state::{ Gateway, Poll, Polls, CONFIG, POLLS, POLL_COUNT};
use secret_toolkit::utils::pad_handle_result;
use tnls::msg::PrivContractHandleMsg;

pub const BLOCK_SIZE: usize = 256;

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {

    let state = Gateway {
        gateway_address: msg.gateway_address,
        gateway_hash: msg.gateway_hash,
        gateway_key: msg.gateway_key,
    };

    CONFIG.save(deps.storage, &state)?;

    deps.api
        .debug(format!("Contract was initialized by {}", info.sender).as_str());

    Ok(Response::default())
}

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    let response = match msg {
        ExecuteMsg::Input { message } => try_handle(deps, env, info, message),
    };
    pad_handle_result(response, BLOCK_SIZE)
}

fn try_handle(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: PrivContractHandleMsg,
) -> StdResult<Response> {
    let gateway_key = CONFIG.load(deps.storage)?.gateway_key;
    deps.api
      .secp256k1_verify(
          msg.input_hash.as_slice(),
          msg.signature.as_slice(),
          gateway_key.as_slice(),
      )
      .map_err(|err| StdError::generic_err(err.to_string()))?;
    let handle = msg.handle.as_str();
    match handle {
        "create_proposal" => try_create_poll(deps, env, msg.input_values),
        "create_vote" => try_vote(deps, env, info, msg.input_values),

        _ => Err(StdError::generic_err("invalid handle".to_string())),
    }
}

pub fn try_create_poll(deps: DepsMut,env: Env, input_values: String ) -> StdResult<Response> {

    let input: CreatePollInput = serde_json_wasm::from_str(&input_values)
        .map_err(|err| StdError::generic_err(err.to_string()))?;
    let mut poll_count=POLL_COUNT.load(deps.storage).unwrap_or(0);
    let mut polls = POLLS
      .load(deps.storage)
      .unwrap_or(Polls {
          polls: Vec::new(),
      });

    polls.polls.push(Poll{
        id: poll_count as u64,
        uri: input.poll_uri,
        created_at: env.block.time,
        validity: input.validity,
        votes: HashMap::new(),
        has_voted: HashMap::new(),
        vote_count: 0,
    });

    poll_count=poll_count+1;

    // Save decrypted votes to storage
    POLLS.save(deps.storage, &polls)?;
    POLL_COUNT.save(deps.storage, &poll_count)?;

    deps.api.debug("poll created successfully");
    Ok(Response::default().add_attribute_plaintext("poll_id", (poll_count-1).to_string()))
}


pub fn try_vote(deps: DepsMut, env: Env, _info: MessageInfo, input_values: String) -> StdResult<Response> {

    let input: VoteInput = serde_json_wasm::from_str(&input_values)
        .map_err(|err| StdError::generic_err(err.to_string()))?;
    let poll_count=POLL_COUNT.load(deps.storage).unwrap_or(0);
    let mut polls = POLLS
      .load(deps.storage)
      .unwrap_or(Polls {
          polls: Vec::new(),
      });
   
    // check if poll exists
    if input.poll_id >= poll_count {
        return Err(StdError::generic_err("Invalid poll id"));
    }

    if let Some(poll) = polls.polls.get_mut(input.poll_id as usize) {
        // check if voting is live
        if env.block.time.seconds() > poll.created_at.seconds() + poll.validity {
            return Err(StdError::generic_err("Voting has ended"));
        }
        
        // check if already voted
        if poll.has_voted.contains_key(&input.farcaster_id) {
            return Err(StdError::generic_err("Already voted")); 
        }
    
        poll.votes.insert(input.vote, poll.votes.get(&input.vote).unwrap_or(&0) + 1);
        poll.has_voted.insert(input.farcaster_id, true);
    
        POLLS.save(deps.storage, &polls)?;
    }else{
        return Err(StdError::generic_err("Poll not found"))

    }

    Ok(Response::default())
}


#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPollCount {} => to_binary(&query_poll_count(deps)?),  
        QueryMsg::GetVoteCount {poll_id} => to_binary(&query_vote_count(deps, poll_id)?),
        QueryMsg::GetResults {poll_id} => to_binary(&query_get_results(deps, poll_id)?),  
        QueryMsg::GetVoted {poll_id, farcaster_id} => to_binary(&query_check_voted(deps, poll_id, farcaster_id)?),
        QueryMsg::GetPoll {poll_id} => to_binary(&query_get_poll(deps, poll_id)?),
    }
}

fn query_poll_count(deps: Deps) -> StdResult<PollCountResponse> {
    let poll_count=POLL_COUNT.load(deps.storage).unwrap_or(0);
    Ok(PollCountResponse { poll_count: poll_count })
}

fn query_vote_count(deps: Deps, poll_id: u64) -> StdResult<VoteCountResponse> {
    let  polls = POLLS
      .load(deps.storage)
      .unwrap_or(Polls {
          polls: Vec::new(),
      });
    let poll = polls.polls.get(poll_id as usize).unwrap();
    Ok(VoteCountResponse { vote_count: poll.vote_count })
}

fn query_check_voted(deps: Deps, poll_id: u64, farcaster_id: u64) -> StdResult<HasVotedResponse> {
    let  polls = POLLS
    .load(deps.storage)
    .unwrap_or(Polls {
        polls: Vec::new(),
    });
  let poll = polls.polls.get(poll_id as usize).unwrap();
  Ok(HasVotedResponse { has_voted: poll.has_voted.contains_key(&farcaster_id)})
}

fn query_get_results(deps: Deps, poll_id: u64) -> StdResult<ResultsResponse> {
    let  polls = POLLS
      .load(deps.storage)
      .unwrap_or(Polls {
          polls: Vec::new(),
      });
    let poll = polls.polls.get(poll_id as usize).unwrap();
    Ok(ResultsResponse {results: poll.votes.clone()})
}


fn query_get_poll(deps: Deps, poll_id: u64) -> StdResult<PollResponse> {
    let polls = POLLS
      .load(deps.storage)
      .unwrap_or(Polls {
          polls: Vec::new(),
      });
    let poll = polls.polls.get(poll_id as usize).unwrap();
    Ok(PollResponse {poll: poll.clone()})
}