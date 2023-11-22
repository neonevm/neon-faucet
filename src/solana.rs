//! Faucet Solana utilities module.

use std::str::FromStr as _;

use eyre::{eyre, Result, WrapErr as _};
use tracing::debug;

use solana_client::rpc_client::RpcClient;
use solana_sdk::instruction::{AccountMeta, Instruction};
use solana_sdk::message::Message;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signer as _;
use solana_sdk::signer::keypair::Keypair;
use solana_sdk::system_program;
use solana_sdk::transaction::Transaction;

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
    let signer_token_pubkey =
        spl_associated_token_account::get_associated_token_address(&signer_pubkey, &token_mint_id);

    let evm_token_authority = Pubkey::find_program_address(&[b"Deposit"], &evm_loader_id).0;
    let evm_pool_pubkey = spl_associated_token_account::get_associated_token_address(
        &evm_token_authority,
        &token_mint_id,
    );

    let ether_balance_pubkey = ether_address_to_balance_pubkey(&evm_loader_id, &ether_address);
    let ether_contract_pubkey = ether_address_to_contract_pubkey(&evm_loader_id, &ether_address);

    let id = id.to_owned();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let client =
            RpcClient::new_with_commitment(config::solana_url(), config::solana_commitment());

        let amount = if in_fractions {
            amount
        } else {
            convert_whole_to_fractions(amount)?
        };

        let instructions = vec![
            spl_memo(&id, &signer_pubkey),
            spl_approve_instruction(
                &id,
                spl_token::id(),
                signer_token_pubkey,
                ether_balance_pubkey,
                signer_pubkey,
                amount,
            ),
            deposit_instruction(
                &id,
                config::solana_chain_id(),
                ether_address,
                token_mint_id,
                signer_token_pubkey,
                evm_pool_pubkey,
                ether_balance_pubkey,
                ether_contract_pubkey,
                evm_loader_id,
                spl_token::id(),
                signer_pubkey,
            ),
        ];

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
fn ether_address_to_balance_pubkey(program_id: &Pubkey, address: &ethereum::Address) -> Pubkey {
    let chain_id = web3::types::U256::from(config::solana_chain_id());

    let mut chain_id_bytes = [0_u8; 32];
    chain_id.to_big_endian(&mut chain_id_bytes);

    let seeds: &[&[u8]] = &[
        &[config::solana_account_seed_version()],
        address.as_bytes(),
        &chain_id_bytes,
    ];
    Pubkey::find_program_address(seeds, program_id).0
}

fn ether_address_to_contract_pubkey(program_id: &Pubkey, address: &ethereum::Address) -> Pubkey {
    let seeds: &[&[u8]] = &[
        &[config::solana_account_seed_version()],
        address.as_bytes(),
    ];
    Pubkey::find_program_address(seeds, program_id).0
}

fn spl_memo(id: &ReqId, pubkey: &Pubkey) -> Instruction {
    debug!("{} Instruction: SPL Memo", id);
    let memo = format!("Neon Faucet {}", id.as_str());
    spl_memo::build_memo(memo.as_bytes(), &[pubkey])
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
#[allow(clippy::too_many_arguments)]
fn deposit_instruction(
    id: &ReqId,
    chain_id: u64,
    ether_address: ethereum::Address,
    mint_pubkey: Pubkey,
    source_pubkey: Pubkey,
    pool_pubkey: Pubkey,
    ether_balance_pubkey: Pubkey,
    ether_contract_pubkey: Pubkey,
    evm_loader_id: Pubkey,
    spl_token_id: Pubkey,
    signer_pubkey: Pubkey,
) -> Instruction {
    debug!("{} Instruction: Deposit", id);

    debug!("{} source_pubkey = {}", id, source_pubkey);
    debug!("{} destination_pubkey = {}", id, pool_pubkey);
    debug!("{} ether_account_pubkey = {}", id, ether_balance_pubkey);

    Instruction::new_with_bincode(
        evm_loader_id,
        &(
            0x31_u8,
            ether_address.as_fixed_bytes(),
            chain_id.to_le_bytes(),
        ),
        vec![
            AccountMeta::new(mint_pubkey, false),
            AccountMeta::new(source_pubkey, false),
            AccountMeta::new(pool_pubkey, false),
            AccountMeta::new(ether_balance_pubkey, false),
            AccountMeta::new(ether_contract_pubkey, false),
            AccountMeta::new_readonly(spl_token_id, false),
            AccountMeta::new(signer_pubkey, true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
    )
}
