use std::collections::HashMap;
use blake2::Blake2s;
use ark_ed_on_bls12_381::EdwardsProjective;
use ark_crypto_primitives::signature::schnorr;
use ark_crypto_primitives::crh::{CRH, pedersen, injective_map::{PedersenCRHCompressor, TECompressor}};
use ark_crypto_primitives::merkle_tree::{self, MerkleTree};


/// Account public key used to verify transaction signatures.
pub type AccountPublicKey = schnorr::PublicKey<EdwardsProjective>;

/// Account ID.
#[derive(Hash, Eq, PartialEq, Copy, Clone, Ord, PartialOrd)]
pub struct AccountId(u8);

/// Transaction amount.
#[derive(Hash, Eq, PartialEq, Copy, Clone)]
pub struct Amount(u64);


#[derive(Hash, Eq, PartialEq, Copy, Clone)]
pub struct AccountInformation {
    public_key: AccountPublicKey,
    balance: Amount
}

impl AccountInformation {
    fn to_bytes(&self) -> Vec<u8> {
        ark_ff::to_bytes![self.public_key, self.balance.0].unwrap()
    }

}

pub struct Parameters {
    pub sig_params: schnorr::Parameters<EdwardsProjective, Blake2s>,
    pub leaf_crh_params: <MerkleTreeCRH as CRH>::Parameters,
    pub two_to_one_crh_params: <MerkleTreeCRH as CRH>::Parameters,
}

pub type MerkleTreeCRH = PedersenCRHCompressor<EdwardsProjective, TECompressor, TwoToOneWindow>;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TwoToOneWindow;

// `WINDOW_SIZE * NUM_WINDOWS` = 2 * 256 bits = enough for hashing two outputs.
impl pedersen::Window for TwoToOneWindow {
    const WINDOW_SIZE: usize = 128;
    const NUM_WINDOWS: usize = 4;
}

pub struct MerkleConfig;
impl merkle_tree::Config for MerkleConfig {
    type LeafHash = MerkleTreeCRH;
    type TwoToOneHash = MerkleTreeCRH;
}

pub struct State {
    pub account_merkle_tree: MerkleTree<MerkleConfig>,
    pub id_to_account_info: HashMap<AccountId, AccountInformation>,
    pub pub_key_to_id: HashMap<schnorr::PublicKey<EdwardsProjective>, AccountId>,
}

impl State {
    /// Create an empty ledger that supports `num_accounts` accounts.
    pub fn new(num_accounts: usize, parameters: &Parameters) -> Self {
        let height = ark_std::log2(num_accounts);
        let account_merkle_tree = MerkleTree::blank(
            &parameters.leaf_crh_params,
            &parameters.two_to_one_crh_params,
            height as usize,
        ).unwrap();
        let pub_key_to_id = HashMap::with_capacity(num_accounts);
        let id_to_account_info = HashMap::with_capacity(num_accounts);
        Self {
            account_merkle_tree,
            id_to_account_info,
            pub_key_to_id,
        }
    }

    /// Create a new account with account identifier `id` and public key `pub_key`.
    /// The initial balance is 0.
    pub fn new_account(&mut self, id: AccountId, public_key: AccountPublicKey) {
        let account_info = AccountInformation {
            public_key,
            balance: Amount(0),
        };
        self.pub_key_to_id.insert(public_key, id);
        self.account_merkle_tree.update(id.0 as usize, &account_info.to_bytes()).expect("should exist");
        self.id_to_account_info.insert(id, account_info);
    }


    /// Update the balance of `id` to `new_amount`.
    /// Returns `Some(())` if an account with identifier `id` exists already, and `None`
    /// otherwise.
    pub fn update_balance(&mut self, id: AccountId, new_amount: Amount) -> Option<()> {
        let tree = &mut self.account_merkle_tree;
        self.id_to_account_info.get_mut(&id).map(|account_info| {
            account_info.balance = new_amount;
            tree.update(id.0 as usize, &account_info.to_bytes()).expect("should exist");
        })
    }
}