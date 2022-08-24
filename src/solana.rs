//! Faucet Solana utilities module.

use std::str::FromStr as _;

use eyre::{eyre, Result, WrapErr as _};
use tracing::{debug, warn};

use solana_client::rpc_client::RpcClient;
use solana_sdk::compute_budget::ComputeBudgetInstruction;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer as _;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::system_program;
use solana_sdk::transaction::Transaction;
use spl_associated_token_account::get_associated_token_address;

use crate::config;
use crate::{ethereum, id::ReqId};

/// Converts amount of tokens from whole value to fractions (usually 10E-9).
pub fn convert_whole_to_fractions(amount: u64) -> Result<u64> {
    let decimals = config::solana_token_mint_decimals();
    let factor = 10_u64
        .checked_pow(decimals as u32)
        .ok_or_else(|| eyre!("Overflow 10^{}", decimals))?;
    amount
        .checked_mul(factor)
        .ok_or_else(|| eyre!("Overflow {}*{}", amount, factor))
}

/// Deposits `amount` of tokens from main account to associated account.
/// When `in_fractions` == false, amount is treated as whole token amount.
/// When `in_fractions` == true, amount is treated as amount in galans (10E-9).
pub async fn deposit_token(
    id: &ReqId,
    signer: Keypair,
    ether_address: ethereum::Address,
    amount: u64,
    in_fractions: bool,
) -> Result<()> {
    let evm_loader_id = Pubkey::from_str(&config::solana_evm_loader()).wrap_err_with(|| {
        eyre!(
            "config::solana_evm_loader returns {}",
            &config::solana_evm_loader()
        )
    })?;
    let token_mint_id = Pubkey::from_str(&config::solana_token_mint_id()).wrap_err_with(|| {
        eyre!(
            "config::solana_token_mint_id returns {}",
            &config::solana_token_mint_id(),
        )
    })?;

    let signer_pubkey = signer.pubkey();
    let signer_token_pubkey = get_associated_token_address(&signer_pubkey, &token_mint_id);

    let evm_token_authority = Pubkey::find_program_address(&[b"Deposit"], &evm_loader_id).0;
    let evm_pool_pubkey = get_associated_token_address(&evm_token_authority, &token_mint_id);

    let ether_pubkey = ether_address_to_solana_pubkey(&ether_address, &evm_loader_id).0;

    let id = id.to_owned();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let client =
            RpcClient::new_with_commitment(config::solana_url(), config::solana_commitment());
        let mut instructions = Vec::with_capacity(6);

        instructions.push(spl_memo(&id, &signer_pubkey));
        instructions.push(compute_budget_instruction_request_heap_frame(&id));
        instructions.push(compute_budget_instruction_set_unit_limit(&id));
        instructions.push(compute_budget_instruction_set_unit_price(&id));

        let ether_account = client.get_account(&ether_pubkey);
        if ether_account.is_err() {
            instructions.push(create_ether_account_instruction(
                &id,
                evm_loader_id,
                signer_pubkey,
                ether_address,
            ));
        }

        let amount = if in_fractions {
            amount
        } else {
            convert_whole_to_fractions(amount)?
        };

        instructions.push(spl_approve_instruction(
            &id,
            spl_token::id(),
            signer_token_pubkey,
            evm_token_authority,
            signer_pubkey,
            amount,
        ));

        instructions.push(deposit_instruction(
            &id,
            signer_token_pubkey,
            evm_pool_pubkey,
            ether_pubkey,
            evm_token_authority,
            evm_loader_id,
            spl_token::id(),
        ));

        debug!(
            "{} Creating message with {} instructions...",
            id,
            instructions.len()
        );
        let message = Message::new(&instructions, Some(&signer_pubkey));
        debug!("{} Creating transaction...", id);
        let mut tx = Transaction::new_unsigned(message);
        debug!("{} Getting latest blockhash...", id);
        let blockhash = client.get_latest_blockhash()?;
        debug!("{} Signing transaction...", id);
        tx.try_sign(&[&signer], blockhash)?;
        debug!("{} Sending and confirming transaction...", id);
        client.send_and_confirm_transaction(&tx)?;
        debug!("{} Transaction is confirmed", id);

        Ok(())
    })
    .await?
}

/// Maps an Ethereum address into a Solana address.
fn ether_address_to_solana_pubkey(
    ether_address: &ethereum::Address,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            &[config::solana_account_seed_version()],
            ether_address.as_bytes(),
        ],
        program_id,
    )
}

fn spl_memo(id: &ReqId, pubkey: &Pubkey) -> Instruction {
    debug!("{} Instruction: SPL Memo", id);
    let memo = format!("Neon Faucet {}", id.as_str());
    spl_memo::build_memo(memo.as_bytes(), &[pubkey])
}

fn compute_budget_instruction_request_heap_frame(id: &ReqId) -> Instruction {
    debug!(
        "{} Instruction: ComputeBudgetInstruction::RequestHeapFrame",
        id
    );
    let hf = config::solana_compute_budget_heap_frame();
    if hf == 0 {
        warn!("{} solana_compute_budget_heap_frame = {}", id, hf);
    } else {
        debug!("{} solana_compute_budget_heap_frame = {}", id, hf);
    }
    ComputeBudgetInstruction::request_heap_frame(hf)
}

fn compute_budget_instruction_set_unit_limit(id: &ReqId) -> Instruction {
    debug!(
        "{} Instruction: ComputeBudgetInstruction::SetComputeUnitLimit",
        id
    );
    let units = config::solana_compute_budget_units();
    if units == 0 {
        warn!("{} solana_compute_budget_units = {}", id, units);
    } else {
        debug!("{} solana_compute_budget_units = {}", id, units);
    }
    ComputeBudgetInstruction::set_compute_unit_limit(units)
}

fn compute_budget_instruction_set_unit_price(id: &ReqId) -> Instruction {
    debug!(
        "{} Instruction: ComputeBudgetInstruction::SetComputeUnitPrice",
        id
    );
    let fee = config::solana_request_units_additional_fee();
    if fee == 0 {
        warn!("{} solana_request_units_additional_fee = {}", id, fee);
    } else {
        debug!("{} solana_request_units_additional_fee = {}", id, fee);
    }
    ComputeBudgetInstruction::set_compute_unit_price(fee)
}

/// Returns instruction for creation of account.
fn create_ether_account_instruction(
    id: &ReqId,
    evm_loader_id: Pubkey,
    signer_pubkey: Pubkey,
    ether_address: ethereum::Address,
) -> Instruction {
    debug!("{} Instruction: CreateAccount", id);
    let (solana_address, nonce) = ether_address_to_solana_pubkey(&ether_address, &evm_loader_id);

    debug!("{} evm_loader_id {}", id, evm_loader_id);
    debug!("{} signer_pubkey {}", id, signer_pubkey);
    debug!("{} ether_address {}", id, ether_address);
    debug!("{} solana_address {}", id, solana_address);

    Instruction::new_with_bincode(
        evm_loader_id,
        &(24_u8, ether_address.as_fixed_bytes(), nonce),
        vec![
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new(solana_address, false),
        ],
    )
}

/// Returns instruction to approve transfer of NEON tokens.
fn spl_approve_instruction(
    id: &ReqId,
    token_program_id: Pubkey,
    source_pubkey: Pubkey,
    delegate_pubkey: Pubkey,
    owner_pubkey: Pubkey,
    amount: u64,
) -> Instruction {
    use spl_token::instruction::TokenInstruction;
    debug!("{} Instruction: TokenInstruction::Approve", id);

    debug!("{} spl_token id = {}", id, token_program_id);
    debug!("{} source_pubkey = {}", id, source_pubkey);
    debug!("{} delegate_pubkey = {}", id, delegate_pubkey);
    debug!("{} owner_pubkey = {}", id, owner_pubkey);
    debug!("{} amount = {}", id, amount);

    let accounts = vec![
        AccountMeta::new(source_pubkey, false),
        AccountMeta::new_readonly(delegate_pubkey, false),
        AccountMeta::new_readonly(owner_pubkey, true),
    ];

    let data = TokenInstruction::Approve { amount }.pack();

    Instruction {
        program_id: token_program_id,
        accounts,
        data,
    }
}

/// Returns instruction to deposit NEON tokens.
fn deposit_instruction(
    id: &ReqId,
    source_pubkey: Pubkey,
    destination_pubkey: Pubkey,
    ether_account_pubkey: Pubkey,
    evm_token_authority: Pubkey,
    evm_loader_id: Pubkey,
    spl_token_id: Pubkey,
) -> Instruction {
    debug!("{} Instruction: Deposit", id);

    debug!("{} source_pubkey = {}", id, source_pubkey);
    debug!("{} destination_pubkey = {}", id, destination_pubkey);
    debug!("{} ether_account_pubkey = {}", id, ether_account_pubkey);
    debug!("{} evm_token_authority = {}", id, evm_token_authority);

    Instruction::new_with_bincode(
        evm_loader_id,
        &(25_u8), // Index of the Deposit instruction in EVM Loader
        vec![
            AccountMeta::new(source_pubkey, false),
            AccountMeta::new(destination_pubkey, false),
            AccountMeta::new(ether_account_pubkey, false),
            AccountMeta::new_readonly(evm_token_authority, false),
            AccountMeta::new_readonly(spl_token_id, false),
        ],
    )
}
