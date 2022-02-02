#![cfg(test)]
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{from_binary, to_binary, CosmosMsg, DepsMut, Response, WasmMsg};

use crate::{
    ApprovedForAllResponse, ContractInfoResponse, Expiration, HighestTokenIdResponse, Metadata,
    MigrateMsg, NftInfoResponse, OwnerOfResponse, ReceiveMsg, TokenId, Trait,
};

use crate::{ContractError, ExecuteMsg, InstantiateMsg, DummyNftContract, MintMsg, QueryMsg};

const MINTER: &str = "merlin";
const CONTRACT_NAME: &str = "Magic Power";
const SYMBOL: &str = "MGK";

fn setup_contract(deps: DepsMut<'_>) -> DummyNftContract<'static> {
    let contract = DummyNftContract::default();
    let msg = InstantiateMsg {
        name: CONTRACT_NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: String::from(MINTER),
    };
    let info = mock_info("creator", &[]);
    let res = contract.instantiate(deps, mock_env(), info, msg).unwrap();
    assert_eq!(0, res.messages.len());
    contract
}

#[test]
fn proper_instantiation() {
    let mut deps = mock_dependencies(&[]);
    let contract = DummyNftContract::default();

    let msg = InstantiateMsg {
        name: CONTRACT_NAME.to_string(),
        symbol: SYMBOL.to_string(),
        minter: String::from(MINTER),
    };
    let info = mock_info("creator", &[]);

    // we can just call .unwrap() to assert this was a success
    let res = contract
        .instantiate(deps.as_mut(), mock_env(), info, msg)
        .unwrap();
    assert_eq!(0, res.messages.len());

    // it worked, let's query the state
    let res = contract.minter(deps.as_ref()).unwrap();
    assert_eq!(MINTER, res.minter);
    let info = contract.contract_info(deps.as_ref()).unwrap();
    assert_eq!(
        info,
        ContractInfoResponse {
            name: CONTRACT_NAME.to_string(),
            symbol: SYMBOL.to_string(),
        }
    );

    let count = contract.num_tokens(deps.as_ref()).unwrap();
    assert_eq!(0, count.count);

    // list the token_ids
    let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
    assert_eq!(0, tokens.tokens.len());
}

#[test]
fn minting() {
    let mut deps = mock_dependencies(&[]);
    let contract = setup_contract(deps.as_mut());

    let token_id = TokenId::new(123);
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();

    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id,
        owner: String::from("medusa"),
        token_uri: Some(token_uri.clone()),
        extension: Metadata::new_test(),
    }));

    // random cannot mint
    let random = mock_info("random", &[]);
    let err = contract
        .execute(deps.as_mut(), mock_env(), random, mint_msg.clone())
        .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // minter can mint
    let allowed = mock_info(MINTER, &[]);
    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed, mint_msg)
        .unwrap();

    // ensure num tokens increases
    let count = contract.num_tokens(deps.as_ref()).unwrap();
    assert_eq!(1, count.count);

    // unknown nft returns error
    let _ = contract
        .nft_info(deps.as_ref(), TokenId::new(99999))
        .unwrap_err();

    // this nft info is correct
    let info = contract.nft_info(deps.as_ref(), token_id.clone()).unwrap();
    assert_eq!(
        info,
        NftInfoResponse {
            token_uri: Some(token_uri),
            extension: Metadata::new_test(),
        }
    );

    // owner info is correct
    let owner = contract
        .owner_of(deps.as_ref(), mock_env(), token_id.clone(), true)
        .unwrap();
    assert_eq!(
        owner,
        OwnerOfResponse {
            owner: String::from("medusa"),
            approvals: vec![],
        }
    );

    // Cannot mint same token_id again
    let mint_msg2 = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id: token_id.clone(),
        owner: String::from("hercules"),
        token_uri: None,
        extension: Metadata::new_test(),
    }));

    let allowed = mock_info(MINTER, &[]);
    let err = contract
        .execute(deps.as_mut(), mock_env(), allowed, mint_msg2)
        .unwrap_err();
    assert_eq!(err, ContractError::Claimed {});

    // list the token_ids
    let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id], tokens.tokens);
}

#[test]
fn burning() {
    let mut deps = mock_dependencies(&[]);
    let contract = setup_contract(deps.as_mut());

    let token_id = TokenId::new(1);
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();

    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id,
        owner: MINTER.to_string(),
        token_uri: Some(token_uri),
        extension: Metadata::new_test(),
    }));

    let burn_msg = ExecuteMsg::Burn { token_id };

    // mint some NFT
    let allowed = mock_info(MINTER, &[]);
    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg)
        .unwrap();

    // random not allowed to burn
    let random = mock_info("random", &[]);
    let err = contract
        .execute(deps.as_mut(), mock_env(), random, burn_msg.clone())
        .unwrap_err();

    assert_eq!(err, ContractError::Unauthorized {});

    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed, burn_msg)
        .unwrap();

    // ensure num tokens decreases
    let count = contract.num_tokens(deps.as_ref()).unwrap();
    assert_eq!(0, count.count);

    // trying to get nft returns error
    let _ = contract.nft_info(deps.as_ref(), token_id).unwrap_err();

    // list the token_ids
    let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
    assert!(tokens.tokens.is_empty());
}

#[test]
fn transferring_nft() {
    let mut deps = mock_dependencies(&[]);
    let contract = setup_contract(deps.as_mut());

    // Mint a token
    let token_id = TokenId::new(101);
    let token_uri = "https://www.merriam-webster.com/dictionary/melt".to_string();

    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id,
        owner: String::from("venus"),
        token_uri: Some(token_uri),
        extension: Metadata::new_test(),
    }));

    let minter = mock_info(MINTER, &[]);
    contract
        .execute(deps.as_mut(), mock_env(), minter, mint_msg)
        .unwrap();

    // random cannot transfer
    let random = mock_info("random", &[]);
    let transfer_msg = ExecuteMsg::TransferNft {
        recipient: String::from("random"),
        token_id: token_id.clone(),
    };

    let err = contract
        .execute(deps.as_mut(), mock_env(), random, transfer_msg)
        .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // owner can
    let random = mock_info("venus", &[]);
    let transfer_msg = ExecuteMsg::TransferNft {
        recipient: String::from("random"),
        token_id: token_id.clone(),
    };

    let res = contract
        .execute(deps.as_mut(), mock_env(), random, transfer_msg)
        .unwrap();

    assert_eq!(
        res,
        Response::new()
            .add_attribute("action", "transfer_nft")
            .add_attribute("sender", "venus")
            .add_attribute("recipient", "random")
            .add_attribute("token_id", token_id)
    );
}

#[test]
fn sending_nft() {
    let mut deps = mock_dependencies(&[]);
    let contract = setup_contract(deps.as_mut());

    // Mint a token
    let token_id = TokenId::new(202);
    let token_uri = "https://www.merriam-webster.com/dictionary/melt".to_string();

    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id,
        owner: String::from("venus"),
        token_uri: Some(token_uri),
        extension: Metadata::new_test(),
    }));

    let minter = mock_info(MINTER, &[]);
    contract
        .execute(deps.as_mut(), mock_env(), minter, mint_msg)
        .unwrap();

    let msg = to_binary("You now have the melting power").unwrap();
    let target = String::from("another_contract");
    let send_msg = ExecuteMsg::SendNft {
        contract: target.clone(),
        token_id,
        msg: msg.clone(),
    };

    let random = mock_info("random", &[]);
    let err = contract
        .execute(deps.as_mut(), mock_env(), random, send_msg.clone())
        .unwrap_err();
    assert_eq!(err, ContractError::Unauthorized {});

    // but owner can
    let random = mock_info("venus", &[]);
    let res = contract
        .execute(deps.as_mut(), mock_env(), random, send_msg)
        .unwrap();

    let payload = ReceiveMsg {
        sender: String::from("venus"),
        token_id,
        msg,
    };
    let expected = payload.into_cosmos_msg(target.clone()).unwrap();
    // ensure expected serializes as we think it should
    match &expected {
        CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, .. }) => {
            assert_eq!(contract_addr, &target)
        }
        m => panic!("Unexpected message type: {:?}", m),
    }
    // and make sure this is the request sent by the contract
    assert_eq!(
        res,
        Response::new()
            .add_message(expected)
            .add_attribute("action", "send_nft")
            .add_attribute("sender", "venus")
            .add_attribute("recipient", "another_contract")
            .add_attribute("token_id", token_id)
    );
}

#[test]
fn approving_revoking() {
    let mut deps = mock_dependencies(&[]);
    let contract = setup_contract(deps.as_mut());

    // Mint a token
    let token_id = TokenId::new(309);
    let token_uri = "https://www.merriam-webster.com/dictionary/grow".to_string();

    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id,
        owner: String::from("demeter"),
        token_uri: Some(token_uri),
        extension: Metadata::new_test(),
    }));

    let minter = mock_info(MINTER, &[]);
    contract
        .execute(deps.as_mut(), mock_env(), minter, mint_msg)
        .unwrap();

    // Give random transferring power
    let approve_msg = ExecuteMsg::Approve {
        spender: String::from("random"),
        token_id,
        expires: None,
    };
    let owner = mock_info("demeter", &[]);
    let res = contract
        .execute(deps.as_mut(), mock_env(), owner, approve_msg)
        .unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_attribute("action", "approve")
            .add_attribute("sender", "demeter")
            .add_attribute("spender", "random")
            .add_attribute("token_id", token_id.clone())
    );

    // random can now transfer
    let random = mock_info("random", &[]);
    let transfer_msg = ExecuteMsg::TransferNft {
        recipient: String::from("person"),
        token_id: token_id.clone(),
    };
    contract
        .execute(deps.as_mut(), mock_env(), random, transfer_msg)
        .unwrap();

    // Approvals are removed / cleared
    let query_msg = QueryMsg::OwnerOf {
        token_id: token_id.clone(),
        include_expired: None,
    };
    let res: OwnerOfResponse = from_binary(
        &contract
            .query(deps.as_ref(), mock_env(), query_msg.clone())
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        res,
        OwnerOfResponse {
            owner: String::from("person"),
            approvals: vec![],
        }
    );

    // Approve, revoke, and check for empty, to test revoke
    let approve_msg = ExecuteMsg::Approve {
        spender: String::from("random"),
        token_id: token_id.clone(),
        expires: None,
    };
    let owner = mock_info("person", &[]);
    contract
        .execute(deps.as_mut(), mock_env(), owner.clone(), approve_msg)
        .unwrap();

    let revoke_msg = ExecuteMsg::Revoke {
        spender: String::from("random"),
        token_id,
    };
    contract
        .execute(deps.as_mut(), mock_env(), owner, revoke_msg)
        .unwrap();

    // Approvals are now removed / cleared
    let res: OwnerOfResponse = from_binary(
        &contract
            .query(deps.as_ref(), mock_env(), query_msg)
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        res,
        OwnerOfResponse {
            owner: String::from("person"),
            approvals: vec![],
        }
    );
}

#[test]
fn approving_all_revoking_all() {
    let mut deps = mock_dependencies(&[]);
    let contract = setup_contract(deps.as_mut());

    // Mint a couple tokens (from the same owner)
    let token_id1 = TokenId::new(1001);
    let token_uri1 = "https://www.merriam-webster.com/dictionary/grow1".to_string();

    let token_id2 = TokenId::new(1002);
    let token_uri2 = "https://www.merriam-webster.com/dictionary/grow2".to_string();

    let mint_msg1 = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id: token_id1,
        owner: String::from("demeter"),
        token_uri: Some(token_uri1),
        extension: Metadata::new_test(),
    }));

    let minter = mock_info(MINTER, &[]);
    contract
        .execute(deps.as_mut(), mock_env(), minter.clone(), mint_msg1)
        .unwrap();

    let mint_msg2 = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id: token_id2.clone(),
        owner: String::from("demeter"),
        token_uri: Some(token_uri2),
        extension: Metadata::new_test(),
    }));

    contract
        .execute(deps.as_mut(), mock_env(), minter, mint_msg2)
        .unwrap();

    // paginate the token_ids
    let tokens = contract.all_tokens(deps.as_ref(), None, Some(1)).unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id1.clone()], tokens.tokens);
    let tokens = contract
        .all_tokens(deps.as_ref(), Some(token_id1), Some(3))
        .unwrap();
    assert_eq!(1, tokens.tokens.len());
    assert_eq!(vec![token_id2.clone()], tokens.tokens);

    // demeter gives random full (operator) power over her tokens
    let approve_all_msg = ExecuteMsg::ApproveAll {
        operator: String::from("random"),
        expires: None,
    };
    let owner = mock_info("demeter", &[]);
    let res = contract
        .execute(deps.as_mut(), mock_env(), owner, approve_all_msg)
        .unwrap();
    assert_eq!(
        res,
        Response::new()
            .add_attribute("action", "approve_all")
            .add_attribute("sender", "demeter")
            .add_attribute("operator", "random")
    );

    // random can now transfer
    let random = mock_info("random", &[]);
    let transfer_msg = ExecuteMsg::TransferNft {
        recipient: String::from("person"),
        token_id: token_id1,
    };
    contract
        .execute(deps.as_mut(), mock_env(), random.clone(), transfer_msg)
        .unwrap();

    // random can now send
    let inner_msg = WasmMsg::Execute {
        contract_addr: "another_contract".into(),
        msg: to_binary("You now also have the growing power").unwrap(),
        funds: vec![],
    };
    let msg: CosmosMsg = CosmosMsg::Wasm(inner_msg);

    let send_msg = ExecuteMsg::SendNft {
        contract: String::from("another_contract"),
        token_id: token_id2,
        msg: to_binary(&msg).unwrap(),
    };
    contract
        .execute(deps.as_mut(), mock_env(), random, send_msg)
        .unwrap();

    // Approve_all, revoke_all, and check for empty, to test revoke_all
    let approve_all_msg = ExecuteMsg::ApproveAll {
        operator: String::from("operator"),
        expires: None,
    };
    // person is now the owner of the tokens
    let owner = mock_info("person", &[]);
    contract
        .execute(deps.as_mut(), mock_env(), owner, approve_all_msg)
        .unwrap();

    let res = contract
        .all_approvals(
            deps.as_ref(),
            mock_env(),
            String::from("person"),
            true,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        res,
        ApprovedForAllResponse {
            operators: vec![crate::Approval {
                spender: String::from("operator"),
                expires: Expiration::Never {}
            }]
        }
    );

    // second approval
    let buddy_expires = Expiration::AtHeight(1234567);
    let approve_all_msg = ExecuteMsg::ApproveAll {
        operator: String::from("buddy"),
        expires: Some(buddy_expires),
    };
    let owner = mock_info("person", &[]);
    contract
        .execute(deps.as_mut(), mock_env(), owner.clone(), approve_all_msg)
        .unwrap();

    // and paginate queries
    let res = contract
        .all_approvals(
            deps.as_ref(),
            mock_env(),
            String::from("person"),
            true,
            None,
            Some(1),
        )
        .unwrap();
    assert_eq!(
        res,
        ApprovedForAllResponse {
            operators: vec![crate::Approval {
                spender: String::from("buddy"),
                expires: buddy_expires,
            }]
        }
    );
    let res = contract
        .all_approvals(
            deps.as_ref(),
            mock_env(),
            String::from("person"),
            true,
            Some(String::from("buddy")),
            Some(2),
        )
        .unwrap();
    assert_eq!(
        res,
        ApprovedForAllResponse {
            operators: vec![crate::Approval {
                spender: String::from("operator"),
                expires: Expiration::Never {}
            }]
        }
    );

    let revoke_all_msg = ExecuteMsg::RevokeAll {
        operator: String::from("operator"),
    };
    contract
        .execute(deps.as_mut(), mock_env(), owner, revoke_all_msg)
        .unwrap();

    // Approvals are removed / cleared without affecting others
    let res = contract
        .all_approvals(
            deps.as_ref(),
            mock_env(),
            String::from("person"),
            false,
            None,
            None,
        )
        .unwrap();
    assert_eq!(
        res,
        ApprovedForAllResponse {
            operators: vec![crate::Approval {
                spender: String::from("buddy"),
                expires: buddy_expires,
            }]
        }
    );

    // ensure the filter works (nothing should be here
    let mut late_env = mock_env();
    late_env.block.height = 1234568; //expired
    let res = contract
        .all_approvals(
            deps.as_ref(),
            late_env,
            String::from("person"),
            false,
            None,
            None,
        )
        .unwrap();
    assert_eq!(0, res.operators.len());
}

#[test]
fn query_tokens_by_owner() {
    let mut deps = mock_dependencies(&[]);
    let contract = setup_contract(deps.as_mut());
    let minter = mock_info(MINTER, &[]);

    // Mint a couple tokens (from the same owner)
    let token_id1 = TokenId::new(1);
    let demeter = String::from("Demeter");
    let token_id2 = TokenId::new(2);
    let ceres = String::from("Ceres");
    let token_id3 = TokenId::new(3);

    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id: token_id1.clone(),
        owner: demeter.clone(),
        token_uri: None,
        extension: Metadata::new_test(),
    }));
    contract
        .execute(deps.as_mut(), mock_env(), minter.clone(), mint_msg)
        .unwrap();

    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id: token_id2.clone(),
        owner: ceres.clone(),
        token_uri: None,
        extension: Metadata::new_test(),
    }));
    contract
        .execute(deps.as_mut(), mock_env(), minter.clone(), mint_msg)
        .unwrap();

    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id: token_id3.clone(),
        owner: demeter.clone(),
        token_uri: None,
        extension: Metadata::new_test(),
    }));
    contract
        .execute(deps.as_mut(), mock_env(), minter, mint_msg)
        .unwrap();

    // get all tokens in order:
    let expected = vec![token_id1.clone(), token_id2.clone(), token_id3.clone()];
    let tokens = contract.all_tokens(deps.as_ref(), None, None).unwrap();
    assert_eq!(&expected, &tokens.tokens);
    // paginate
    let tokens = contract.all_tokens(deps.as_ref(), None, Some(2)).unwrap();
    assert_eq!(&expected[..2], &tokens.tokens[..]);
    let tokens = contract
        .all_tokens(deps.as_ref(), Some(expected[1].clone()), None)
        .unwrap();
    assert_eq!(&expected[2..], &tokens.tokens[..]);

    // get by owner
    let by_ceres = vec![token_id2];
    let by_demeter = vec![token_id1, token_id3];
    // all tokens by owner
    let tokens = contract
        .tokens(deps.as_ref(), demeter.clone(), None, None)
        .unwrap();
    assert_eq!(&by_demeter, &tokens.tokens);
    let tokens = contract.tokens(deps.as_ref(), ceres, None, None).unwrap();
    assert_eq!(&by_ceres, &tokens.tokens);

    // paginate for demeter
    let tokens = contract
        .tokens(deps.as_ref(), demeter.clone(), None, Some(1))
        .unwrap();
    assert_eq!(&by_demeter[..1], &tokens.tokens[..]);
    let tokens = contract
        .tokens(deps.as_ref(), demeter, Some(by_demeter[0].clone()), Some(3))
        .unwrap();
    assert_eq!(&by_demeter[1..], &tokens.tokens[..]);
}

#[test]
fn use_metadata_extension() {
    const CREATOR: &str = "creator";

    let mut deps = mock_dependencies(&[]);
    let contract = DummyNftContract::default();

    let info = mock_info(CREATOR, &[]);
    let init_msg = InstantiateMsg {
        name: "SpaceShips".to_string(),
        symbol: "SPACE".to_string(),
        minter: CREATOR.to_string(),
    };
    contract
        .instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg)
        .unwrap();

    let token_id = TokenId::new(1701);
    let mint_msg = MintMsg {
        token_id,
        owner: "john".to_string(),
        token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
        extension: Metadata::new_test(),
    };
    let exec_msg = ExecuteMsg::Mint(Box::new(mint_msg.clone()));
    contract
        .execute(deps.as_mut(), mock_env(), info, exec_msg)
        .unwrap();

    let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
    assert_eq!(res.token_uri, mint_msg.token_uri);
    assert_eq!(res.extension, mint_msg.extension);
}

#[test]
fn burn_and_reuse() {
    let mut deps = mock_dependencies(&[]);
    let contract = setup_contract(deps.as_mut());

    let token_id = TokenId::new(1);
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();

    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id,
        owner: MINTER.to_string(),
        token_uri: Some(token_uri),
        extension: Metadata::new_test(),
    }));

    let burn_msg = ExecuteMsg::Burn { token_id };

    // mint some NFT
    let allowed = mock_info(MINTER, &[]);
    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg.clone())
        .unwrap();

    // Burn it
    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed.clone(), burn_msg)
        .unwrap();

    // Cannot remint
    let err = contract
        .execute(deps.as_mut(), mock_env(), allowed, mint_msg)
        .unwrap_err();

    assert_eq!(err, ContractError::RemintBurned { token_id });
}

#[test]
fn highest_token_id() {
    let mut deps = mock_dependencies(&[]);
    let contract = setup_contract(deps.as_mut());

    // Starts as None
    assert_eq!(
        contract.highest_token_id(deps.as_ref()).unwrap(),
        HighestTokenIdResponse {
            highest_token_id: None
        }
    );

    let token_id = TokenId::new(1);
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();
    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id,
        owner: MINTER.to_string(),
        token_uri: Some(token_uri),
        extension: Metadata::new_test(),
    }));
    let allowed = mock_info(MINTER, &[]);
    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg.clone())
        .unwrap();

    // ID has been updated
    assert_eq!(
        contract.highest_token_id(deps.as_ref()).unwrap(),
        HighestTokenIdResponse {
            highest_token_id: Some(token_id)
        }
    );

    // Burning does not affect it
    let burn_msg = ExecuteMsg::Burn { token_id };
    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed.clone(), burn_msg)
        .unwrap();
    assert_eq!(
        contract.highest_token_id(deps.as_ref()).unwrap(),
        HighestTokenIdResponse {
            highest_token_id: Some(token_id),
        }
    );

    // Minting again updates
    let token_id = TokenId::new(5);
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();
    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id,
        owner: MINTER.to_string(),
        token_uri: Some(token_uri),
        extension: Metadata::new_test(),
    }));
    let allowed = mock_info(MINTER, &[]);
    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg.clone())
        .unwrap();
    assert_eq!(
        contract.highest_token_id(deps.as_ref()).unwrap(),
        HighestTokenIdResponse {
            highest_token_id: Some(token_id),
        }
    );

    // Minting lower has no affect updates
    let lower_token_id = TokenId::new(4);
    let token_uri = "https://www.merriam-webster.com/dictionary/petrify".to_string();
    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id: lower_token_id,
        owner: MINTER.to_string(),
        token_uri: Some(token_uri),
        extension: Metadata::new_test(),
    }));
    let allowed = mock_info(MINTER, &[]);
    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg.clone())
        .unwrap();
    assert_eq!(
        contract.highest_token_id(deps.as_ref()).unwrap(),
        HighestTokenIdResponse {
            highest_token_id: Some(token_id),
        }
    );
}


#[test]
fn can_migrate() {
    let mut deps = mock_dependencies(&[]);
    let contract = setup_contract(deps.as_mut());

    const NEW_MINTER: &str = "newminter";
    let allowed = mock_info(MINTER, &[]);
    let next_allowed = mock_info(NEW_MINTER, &[]);

    // Next minter can't mint, original one can
    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id: TokenId::new(945),
        owner: String::from("someowner"),
        token_uri: None,
        extension: Metadata::new_test(),
    }));
    let _ = contract
        .execute(
            deps.as_mut(),
            mock_env(),
            next_allowed.clone(),
            mint_msg.clone(),
        )
        .unwrap_err();
    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg)
        .unwrap();

    // Perform the migration
    const NEW_NAME: &str = "newname";
    const NEW_SYMBOL: &str = "newsymbol";
    let migrate_msg = MigrateMsg {
        name: Some(NEW_NAME.to_owned()),
        symbol: Some(NEW_SYMBOL.to_owned()),
        minter: Some(NEW_MINTER.to_owned()),
    };
    let _ = contract
        .migrate(deps.as_mut(), migrate_msg.clone())
        .unwrap();

    // Ensure new metadata
    let info = contract.contract_info(deps.as_ref()).unwrap();
    assert_eq!(
        info,
        ContractInfoResponse {
            name: NEW_NAME.to_owned(),
            symbol: NEW_SYMBOL.to_owned()
        }
    );

    // Next minter can mint, original one can't
    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id: TokenId::new(946),
        owner: String::from("someowner"),
        token_uri: None,
        extension: Metadata::new_test(),
    }));
    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed.clone(), mint_msg.clone())
        .unwrap_err();
    let _ = contract
        .execute(deps.as_mut(), mock_env(), next_allowed.clone(), mint_msg)
        .unwrap();

    // Perform another migration
    const NEW_NEW_NAME: &str = "newnewname";
    let migrate_msg = MigrateMsg {
        name: Some(NEW_NEW_NAME.to_owned()),
        symbol: None,
        minter: Some(MINTER.to_owned()),
    };
    let _ = contract
        .migrate(deps.as_mut(), migrate_msg.clone())
        .unwrap();

    // Ensure new metadata
    let info = contract.contract_info(deps.as_ref()).unwrap();
    assert_eq!(
        info,
        ContractInfoResponse {
            name: NEW_NEW_NAME.to_owned(),
            symbol: NEW_SYMBOL.to_owned()
        }
    );

    // Next minter can't mint, original one can
    let mint_msg = ExecuteMsg::Mint(Box::new(MintMsg {
        token_id: TokenId::new(947),
        owner: String::from("someowner"),
        token_uri: None,
        extension: Metadata::new_test(),
    }));
    let _ = contract
        .execute(deps.as_mut(), mock_env(), next_allowed, mint_msg.clone())
        .unwrap_err();
    let _ = contract
        .execute(deps.as_mut(), mock_env(), allowed, mint_msg)
        .unwrap();
}
