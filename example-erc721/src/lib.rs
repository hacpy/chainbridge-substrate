// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

mod mock;
mod tests;

use codec::{Decode, Encode};
use frame_support::{ensure};
use sp_core::U256;
use sp_runtime::RuntimeDebug;
use sp_std::prelude::*;

pub use pallet::*;
use frame_support::dispatch::DispatchResultWithPostInfo;

type TokenId = U256;

#[derive(PartialEq, Eq, Clone, Encode, Decode, RuntimeDebug)]
pub struct Erc721Token {
    pub id: TokenId,
    pub metadata: Vec<u8>,
}

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use super::*;

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    // NOTE: if the visibility of trait store is private but you want to make it available
    // in super, then use `pub(super)` or `pub(crate)` to make it available in crate.
    pub struct Pallet<T>(_);
    // pub struct Pallet<T, I = ()>(PhantomData<T>); // for instantiable pallet

    #[pallet::config]
    pub trait Config: frame_system::Config {

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// Some identifier for this token type, possibly the originating ethereum address.
        /// This is not explicitly used for anything, but may reflect the bridge's notion of resource ID.
        type Identifier: Get<[u8; 32]>;
    }


    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(<T as frame_system::Config>::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        /// New token created
        Minted(<T as frame_system::Config>::AccountId, TokenId),
        /// Token transfer between two parties
        Transferred(<T as frame_system::Config>::AccountId, <T as frame_system::Config>::AccountId, TokenId),
        /// Token removed from the system
        Burned(TokenId),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// ID not recognized
        TokenIdDoesNotExist,
        /// Already exists with an owner
        TokenAlreadyExists,
        /// Origin is not owner
        NotOwner,
    }

    #[pallet::storage]
    #[pallet::getter(fn tokens)]
    pub(super) type Tokens<T: Config> = StorageMap<
        _,
        Blake2_256,
        TokenId,
        Erc721Token
    >;

    #[pallet::storage]
    #[pallet::getter(fn owner_of)]
    pub(super) type TokenOwner<T: Config> = StorageMap<
        _,
        Blake2_256,
        TokenId,
        T::AccountId
    >;


    #[pallet::type_value]
    pub(super) fn TokenCountDefault<T: Config>() -> U256 {
        U256::zero()
    }

    #[pallet::storage]
    #[pallet::getter(fn token_count)]
    pub(super) type TokenCount<T: Config> = StorageValue<_, U256, ValueQuery, TokenCountDefault<T>>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {

    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Creates a new token with the given token ID and metadata, and gives ownership to owner
        #[pallet::weight(195_000_000)]
        pub fn mint(
            origin: OriginFor<T>,
            owner: T::AccountId, id: TokenId, metadata: Vec<u8>
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            Self::mint_token(owner, id, metadata)?;

            Ok(().into())
        }

        /// Changes ownership of a token sender owns
        #[pallet::weight(195_000_000)]
        pub fn transfer(
            origin: OriginFor<T>,
            to: T::AccountId,
            id: TokenId
        ) -> DispatchResultWithPostInfo {
            let sender = ensure_signed(origin)?;

            Self::transfer_from(sender, to, id)?;

            Ok(().into())
        }

        /// Remove token from the system
        #[pallet::weight(195_000_000)]
        pub fn burn(
            origin: OriginFor<T>,
            id: TokenId
        ) -> DispatchResultWithPostInfo {
            ensure_root(origin)?;

            let owner = Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;

            Self::burn_token(owner, id)?;

            Ok(().into())
        }
    }
}

impl<T: Config> Pallet<T> {
    /// Creates a new token in the system.
    pub fn mint_token(
        owner: T::AccountId,
        id: TokenId,
        metadata: Vec<u8>
    ) -> DispatchResultWithPostInfo {
        ensure!(!<Tokens<T>>::contains_key(id), Error::<T>::TokenAlreadyExists);

        let new_token = Erc721Token { id, metadata };

        <Tokens<T>>::insert(&id, new_token);
        <TokenOwner<T>>::insert(&id, owner.clone());
        let new_total = <TokenCount<T>>::get().saturating_add(U256::one());
        <TokenCount<T>>::put(new_total);

        Self::deposit_event(Event::Minted(owner, id));

        Ok(().into())
    }

    /// Modifies ownership of a token
    pub fn transfer_from(
        from: T::AccountId,
        to: T::AccountId,
        id: TokenId
    ) -> DispatchResultWithPostInfo {
        // Check from is owner and token exists
        let owner = Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;
        ensure!(owner == from, Error::<T>::NotOwner);
        // Update owner
        <TokenOwner<T>>::insert(&id, to.clone());

        Self::deposit_event(Event::Transferred(from, to, id));

        Ok(().into())
    }

    /// Deletes a token from the system.
    pub fn burn_token(
        from: T::AccountId,
        id: TokenId
    ) -> DispatchResultWithPostInfo {
        let owner = Self::owner_of(id).ok_or(Error::<T>::TokenIdDoesNotExist)?;
        ensure!(owner == from, Error::<T>::NotOwner);

        <Tokens<T>>::remove(&id);
        <TokenOwner<T>>::remove(&id);
        let new_total = <TokenCount<T>>::get().saturating_add(U256::one());
        <TokenCount<T>>::put(new_total);

        Self::deposit_event(Event::Burned(id));

        Ok(().into())
    }
}