# Deposit Handler 

## Overview  

Write a smart contract that allows users to deposit 2 tokens in a 1:1 ratio, and deposits those tokens into two separate strategy contracts, one per deposit denom.   

Deposits are tracked using an ID, and the address which set up this ID first becomes the owner of the ID: all calls using this ID must be done by the ID owner.   

Tokens allowed for deposits are defined in the Config, as well as the unbond time for the assets.  

## Logic Flow   

The process is the following
- Deposit funds using the Bond message. Funds must be the two allowed denoms, with the amount of funds on both side being equal (so 1:1).   
- The funds are sent to target contracts specified in the config (one contract per denom). These return a callback confirming that the assets have been bonded.  
- The user can then ask for unbonding of his / her assets by using the StartUnbond message. This message will notify the target contracts that they should start unbonding assets. Once the start of the unbonding is successful, the target contracts notify the base contract that the unbonding has started, through the StartUnbondResponse callback. TO NOTE: for a given asset, there can only be one StartUnbond at a time per ID. Once the Deposit Handler contract has received the StartUnbondResponse callback from the 2 subcontracts, the user can ask for more StartUnbond again.  
- Once the lock period is over, the user can call Unbond to get its funds back. The Deposit Handler contract sends a notification to the target contracts that the user is requesting the return of its available funds. Funds will then be returned in the UnbondResponse callbacks.   


## Internals  

State is managed on a per ID basis in a "BondStatus" object.  

```rust 
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, Default)]
pub struct BondStatus {
    pub sent_to_bond: BondStatusData,
    pub bonded: BondStatusData,
    pub unconfirmed_unbonding: BondStatusData,
    pub unbonding: Vec<UnbondingElement>,
    pub sent_for_unbond: BondStatusData,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, Default)]
pub struct BondStatusData {
    pub denom_1: Uint128,
    pub denom_2: Uint128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema, Default)]
pub struct UnbondingElement {
    pub denom: String,
    pub value: Uint128,
    pub unbonding_start_time: Timestamp,
}
```

This allows to track the process step by step. Let's take an action per action look.

### User calls Bond  
Fund amounts are added to the sent_to_bond field. On the callback BondResponse, the share amount is available so sent_to_bond is decreased appropriately, and share_amount is then added to the bonded field.  


### User calls StartUnbond  
If bonded funds are enough, the value of bonded assets is diminished by the share_amount of the call and these unconfirmed start unbonds are tracked in the unconfirmed unbonding field.    
On the callback StartUnbondResponse, unconfirmed unbonding is set to 0 for the relevant denom, and we create an unbonding element that's added the unbonding field, which tracks all ongoing unbonding. 

This works since we actually prevent 2 StartUnbond at the same time: a user can only do another StartUnbond for a given ID if the previous one has been handled successfully. 

### User calls Unbond  
We compute the total of funds available for Unbond using the elements in the unbonding field, under constraints of time of start unbond and lock time for unbond.   
In the state, unbonding elements are represented by a Vector of unbonding elements, not a total like other fields. 
In this case, we iterate on these elements and consume them (partially or entirely), and then add the value of the request unbond to the sent_for_unbond field. 

Then the deposit handler sends a request for the funds to the target contract. On the UnbondResponse callback, using the funds value of the message we decrease the values of the field sent_for_unbond, and then send the funds to the target user.  




