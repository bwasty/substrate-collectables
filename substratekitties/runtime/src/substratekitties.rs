#![allow(clippy::redundant_closure)]

use parity_codec::{Decode, Encode};
use runtime_primitives::traits::{As, Hash, Zero};
use support::{
    decl_event, decl_module, decl_storage, dispatch::Result, ensure, traits::Currency, StorageMap,
    StorageValue,
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
        <T as system::Trait>::Hash,
        <T as balances::Trait>::Balance
    {
        Created(AccountId, Hash),
        PriceSet(AccountId, Hash, Balance),
        Transferred(AccountId, AccountId, Hash),
        Bought(AccountId, AccountId, Hash, Balance),
    }
);

decl_storage! {
    trait Store for Module<T: Trait> as KittyStorage {
        Kitties get(kitty): map T::Hash => Kitty<T::Hash, T::Balance>;
        KittyOwner get(owner_of): map T::Hash => Option<T::AccountId>;

        AllKittiesArray get(kitty_by_index): map u64 => T::Hash;
        AllKittiesCount get(all_kitties_count): u64;
        AllKittiesIndex: map T::Hash => u64;

        OwnedKittiesArray get(kitty_of_owner_by_index): map (T::AccountId, u64) => T::Hash;
        OwnedKittiesCount get(owned_kitty_count): map T::AccountId => u64;
        OwnedKittiesIndex: map T::Hash => u64;

        Nonce: u64;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn deposit_event<T>() = default;

        fn create_kitty(origin) -> Result {
            let sender = ensure_signed(origin)?;

            let nonce = <Nonce<T>>::get();
            let random_hash = (<system::Module<T>>::random_seed(), &sender, nonce)
                .using_encoded(<T as system::Trait>::Hashing::hash);

            let new_kitty = Kitty {
                id: random_hash,
                dna: random_hash,
                price: <T::Balance as As<u64>>::sa(0),
                gen: 0,
            };

            Self::mint(sender, random_hash, new_kitty)?;
            <Nonce<T>>::mutate(|n| *n += 1);

            Ok(())
        }

        fn set_price(origin, kitty_id: T::Hash, new_price: T::Balance) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(Kitties::<T>::exists(kitty_id), "kitty doesn't exist");

            let owner = Self::owner_of(kitty_id).ok_or("kitty owner doesn't exist")?;
            ensure!(owner == sender, "You are not the owner of this kitty");

            let mut kitty = Self::kitty(kitty_id);

            kitty.price = new_price;

            Kitties::<T>::insert(kitty.id, kitty);

            Self::deposit_event(RawEvent::PriceSet(sender, kitty_id, new_price));

            Ok(())
        }

        fn transfer(origin, to: T::AccountId, kitty_id: T::Hash) -> Result {
            let sender = ensure_signed(origin)?;

            let owner = Self::owner_of(kitty_id).ok_or("No owner for this kitty")?;

            ensure!(owner == sender, "You do not own this kitty");

            Self::transfer_from(sender, to, kitty_id)?;

            Ok(())
        }

        fn buy_kitty(origin, kitty_id: T::Hash, max_price: T::Balance) -> Result {
            let sender = ensure_signed(origin)?;

            ensure!(Kitties::<T>::exists(kitty_id), "kitty doesn't exist");

            let owner = Self::owner_of(kitty_id).ok_or("kitty owner doesn't exist")?;
            ensure!(owner != sender, "You already are the owner of this kitty");

            let mut kitty = Self::kitty(kitty_id);

            let price = kitty.price;
            ensure!(!price.is_zero(), "kitty is not for sale");

            ensure!(price <= max_price, "price is greater than max_price");

            <balances::Module<T> as Currency<_>>::transfer(&owner, &sender, price)?;

            Self::transfer_from(owner.clone(), sender.clone(), kitty_id)
            .expect("`owner` is shown to own the kitty; \
            `owner` must have greater than 0 kitties, so transfer cannot cause underflow; \
            `all_kitty_count` shares the same type as `owned_kitty_count` \
            and minting ensure there won't ever be more than `max()` kitties, \
            which means transfer cannot cause an overflow; \
            qed");

            kitty.price = <T::Balance as As<u64>>::sa(0);
            Kitties::<T>::insert(kitty.id, kitty);

            Self::deposit_event(RawEvent::Bought(sender, owner, kitty_id, price));

            Ok(())
        }

    }
}

impl<T: Trait> Module<T> {
    fn mint(to: T::AccountId, kitty_id: T::Hash, new_kitty: Kitty<T::Hash, T::Balance>) -> Result {
        ensure!(!<Kitties<T>>::exists(kitty_id), "Kitty already exists");

        let owned_kitty_count = Self::owned_kitty_count(&to);
        let new_owned_kitty_count = owned_kitty_count
            .checked_add(1)
            .ok_or("Overflow adding a new kitty to owner")?;

        let all_kitties_count = Self::all_kitties_count();
        // let all_kitties_count = AllKittiesCount::<T>::get(); // equivalent
        let new_all_kitties_count = all_kitties_count
            .checked_add(1)
            .ok_or("Overflow adding a new kitty to total supply")?;

        Kitties::<T>::insert(kitty_id, new_kitty.clone());
        <KittyOwner<T>>::insert(kitty_id, &to);

        AllKittiesArray::<T>::insert(all_kitties_count, kitty_id);
        AllKittiesCount::<T>::put(new_all_kitties_count);
        AllKittiesIndex::<T>::insert(kitty_id, all_kitties_count);

        <OwnedKittiesArray<T>>::insert((to.clone(), owned_kitty_count), kitty_id);
        OwnedKittiesCount::<T>::insert(&to, new_owned_kitty_count);
        OwnedKittiesIndex::<T>::insert(kitty_id, owned_kitty_count);

        Self::deposit_event(RawEvent::Created(to, kitty_id));

        Ok(())
    }

    fn transfer_from(from: T::AccountId, to: T::AccountId, kitty_id: T::Hash) -> Result {
        let owner = Self::owner_of(kitty_id).ok_or("kitty has no owner")?;

        ensure!(owner == from, "You are not the owner of this kitty");

        let owned_kitty_count_from = Self::owned_kitty_count(&from);
        let owned_kitty_count_to = Self::owned_kitty_count(&to);

        let new_owned_kitty_count_to = owned_kitty_count_to.checked_add(1).ok_or("int overflow")?;
        let new_owned_kitty_count_from = owned_kitty_count_from
            .checked_sub(1)
            .ok_or("int underflow")?;

        // NOTE: This is the "swap and pop" algorithm we have added for you
        //       We use our storage items to help simplify the removal of elements from the OwnedKittiesArray
        //       We switch the last element of OwnedKittiesArray with the element we want to remove
        let kitty_index = <OwnedKittiesIndex<T>>::get(kitty_id);
        if kitty_index != new_owned_kitty_count_from {
            let last_kitty_id =
                <OwnedKittiesArray<T>>::get((from.clone(), new_owned_kitty_count_from));
            <OwnedKittiesArray<T>>::insert((from.clone(), kitty_index), last_kitty_id);
            <OwnedKittiesIndex<T>>::insert(last_kitty_id, kitty_index);
        }
        // Now we can remove this item by removing the last element

        KittyOwner::<T>::insert(&kitty_id, &to);
        OwnedKittiesIndex::<T>::insert(kitty_id, owned_kitty_count_to);

        OwnedKittiesArray::<T>::remove((from.clone(), new_owned_kitty_count_from));
        OwnedKittiesArray::<T>::insert((to.clone(), owned_kitty_count_to), kitty_id);

        OwnedKittiesCount::<T>::insert(&from, new_owned_kitty_count_from);
        OwnedKittiesCount::<T>::insert(&to, new_owned_kitty_count_to);

        Self::deposit_event(RawEvent::Transferred(from, to, kitty_id));

        Ok(())
    }
}
