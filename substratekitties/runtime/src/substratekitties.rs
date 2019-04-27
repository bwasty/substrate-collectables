#![allow(clippy::redundant_closure)]

use parity_codec::{Decode, Encode};
use runtime_primitives::traits::{As, Hash};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, StorageMap, StorageValue,
};
use system::ensure_signed;

#[derive(Encode, Decode, Default, Clone, PartialEq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Kitty<Hash, Balance> {
    id: Hash,
    dna: Hash,
    price: Balance,
    gen: u64,
}

pub trait Trait: balances::Trait {
    type Event: From<Event<Self>> + Into<<Self as system::Trait>::Event>;
}

decl_event!(
    pub enum Event<T>
    where
        <T as system::Trait>::AccountId,
        <T as system::Trait>::Hash
    {
        Created(AccountId, Hash),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as KittyStorage {
        Kitties: map T::Hash => Kitty<T::Hash, T::Balance>;
        KittyOwner: map T::Hash => Option<T::AccountId>;

        AllKittiesArray get(kitty_by_index): map u64 => T::Hash;
        AllKittiesCount get(all_kitties_count): u64;
        AllKittiesIndex: map T::Hash => u64;

        OwnedKitty get(kitty_of_owner): map T::AccountId => T::Hash;

        Nonce: u64;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        fn create_kitty(origin) -> Result {
            let sender = ensure_signed(origin)?;

            let all_kitties_count = Self::all_kitties_count();
            // let all_kitties_count = AllKittiesCount::<T>::get(); // equivalent
            let new_all_kitties_count = all_kitties_count.checked_add(1).ok_or("Overflow adding a new kitty to total supply")?;

            let nonce = <Nonce<T>>::get();
            let random_hash = (<system::Module<T>>::random_seed(), &sender, nonce)
                .using_encoded(<T as system::Trait>::Hashing::hash);

            ensure!(!<Kitties<T>>::exists(random_hash), "Kitty already exists");

            let new_kitty = Kitty {
                id: random_hash,
                dna: random_hash,
                price: <T::Balance as As<u64>>::sa(0),
                gen: 0,
            };

            Kitties::<T>::insert(random_hash, new_kitty.clone());
            <KittyOwner<T>>::insert(&random_hash, &sender);

            AllKittiesArray::<T>::insert(all_kitties_count, random_hash);
            AllKittiesCount::<T>::put(new_all_kitties_count);
            AllKittiesIndex::<T>::insert(random_hash, all_kitties_count);

            <OwnedKitty<T>>::insert(&sender, random_hash);

            <Nonce<T>>::mutate(|n| *n += 1);

            Self::deposit_event(RawEvent::Created(sender, random_hash));

            Ok(())
        }
    }
}
