use chainbridge as bridge;
use example_erc721 as erc721;
use frame_support::traits::{Currency, EnsureOrigin, ExistenceRequirement::AllowDeath, Get};
use frame_support::{decl_error, decl_event, decl_module, dispatch::DispatchResult, ensure};
use frame_system::{self as system, ensure_signed};
use sp_arithmetic::traits::SaturatedConversion;
use sp_core::U256;
use sp_std::prelude::*;

mod mock;
mod tests;

pub use pallet::*;

#[frame_support::pallet]
pub mod pallet {
    use frame_support::pallet_prelude::*;
    use frame_system::pallet_prelude::*;
    use super::*;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::Hash = "Hash")]
    pub enum Event<T> {
        Remark(T::Hash),
    }

    #[pallet::error]
    pub enum Error<T> {
        InvalidTransfer,
    }

    #[pallet::config]
    pub trait Config: frame_system::Config {
        #[pallet::constant]
        type ResourceId: Get<bridge::ResourceId>;

        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// Specifies the origin check provided by the bridge for calls that can only be called by the bridge pallet
        type BridgeOrigin: EnsureOrigin<Self::Origin, Success = Self::AccountId>;

        /// The currency mechanism.
        type Currency: Currency<Self::AccountId>;

        /// Ids can be defined by the runtime and passed in, perhaps from blake2b_128 hashes.
        type HashId: Get<ResourceId>;
        type NativeTokenId: Get<ResourceId>;
        type Erc721Id: Get<ResourceId>;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    // NOTE: if the visibility of trait store is private but you want to make it available
    // in super, then use `pub(super)` or `pub(crate)` to make it available in crate.
    pub struct Pallet<T>(_);
    // pub struct Pallet<T, I = ()>(PhantomData<T>); // for instantiable pallet

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumber> for Pallet<T> {

    }

    #[pallet::call]
    impl<T: Config> Pallet<T> {
        /// Transfers an arbitrary hash to a (whitelisted) destination chain.
        #[pallet::weight(195_000_000)]
        pub fn transfer_hash(
            origin: OriginFor<T>,
            hash: T::Hash,
            dest_id: bridge::ChainId
        ) -> DispatchResultWithPostInfo {
            ensure_signed(origin)?;

            let resource_id = T::HashId::get();
            let metadata: Vec<u8> = hash.as_ref().to_vec();
            <bridge::Module<T>>::transfer_generic(dest_id, resource_id, metadata)
        }

        /// Transfers some amount of the native token to some recipient on a (whitelisted) destination chain.
        #[pallet::weight(195_000_000)]
        pub fn transfer_native(
            origin: OriginFor<T>,
            amount: BalanceOf<T>,
            recipient: Vec<u8>,
            dest_id: bridge::ChainId
        ) -> DispatchResultWithPostInfo {
            let source = ensure_signed(origin)?;
            ensure!(<bridge::Module<T>>::chain_whitelisted(dest_id), Error::<T>::InvalidTransfer);
            let bridge_id = <bridge::Module<T>>::account_id();
            T::Currency::transfer(&source, &bridge_id, amount.into(), AllowDeath)?;

            let resource_id = T::NativeTokenId::get();
            <bridge::Module<T>>::transfer_fungible(dest_id, resource_id, recipient, U256::from(amount.saturated_into::<u128>()))
        }

        /// Transfer a non-fungible token (erc721) to a (whitelisted) destination chain.
        #[pallet::weight(195_000_000)]
        pub fn transfer_erc721(
            origin: OriginFor<T>,
            recipient: Vec<u8>,
            token_id: U256,
            dest_id: bridge::ChainId
        ) -> DispatchResultWithPostInfo {
            let source = ensure_signed(origin)?;
            ensure!(<bridge::Module<T>>::chain_whitelisted(dest_id), Error::<T>::InvalidTransfer);
            match <erc721::Module<T>>::tokens(&token_id) {
                Some(token) => {
                    <erc721::Module<T>>::burn_token(source, token_id)?;
                    let resource_id = T::Erc721Id::get();
                    let tid: &mut [u8] = &mut[0; 32];
                    token_id.to_big_endian(tid);
                    <bridge::Module<T>>::transfer_nonfungible(dest_id, resource_id, tid.to_vec(), recipient, token.metadata)
                }
                None => Err(Error::<T>::InvalidTransfer)?
            }
        }

        //
        // Executable calls. These can be triggered by a bridge transfer initiated on another chain
        //

        /// Executes a simple currency transfer using the bridge account as the source
        #[pallet::weight(195_000_000)]
        pub fn transfer(
            origin: OriginFor<T>,
            to: T::AccountId,
            amount: BalanceOf<T>,
            r_id: ResourceId
        ) -> DispatchResultWithPostInfo {
            let source = T::BridgeOrigin::ensure_origin(origin)?;
            <T as Config>::Currency::transfer(&source, &to, amount.into(), AllowDeath)?;
            Ok(().into())
        }

        /// This can be called by the bridge to demonstrate an arbitrary call from a proposal.
        #[pallet::weight(195_000_000)]
        pub fn remark(
            origin: OriginFor<T>,
            hash: T::Hash,
            r_id: ResourceId
        ) -> DispatchResultWithPostInfo {
            T::BridgeOrigin::ensure_origin(origin)?;
            Self::deposit_event(RawEvent::Remark(hash));
            Ok(().into())
        }

        /// Allows the bridge to issue new erc721 tokens
        #[pallet::weight(195_000_000)]
        pub fn mint_erc721(
            origin: Origin<T>,
            recipient: T::AccountId,
            id: U256,
            metadata: Vec<u8>,
            r_id: ResourceId
        ) -> DispatchResultWithPostInfo {
            T::BridgeOrigin::ensure_origin(origin)?;
            <erc721::Module<T>>::mint_token(recipient, id, metadata)?;
            Ok(().into())
        }
    }
}