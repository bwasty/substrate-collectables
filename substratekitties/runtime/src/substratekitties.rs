use support::{decl_module, decl_storage, dispatch::Result, StorageMap};
use system::ensure_signed;

pub trait Trait: balances::Trait {}

#[allow(clippy::redundant_closure)]
decl_storage! {
    trait Store for Module<T: Trait> as KittyStorage {
        Value: map T::AccountId => u64;
    }
}

decl_module! {
    pub struct Module<T: Trait> for enum Call where origin: T::Origin {
        fn set_value(origin, value: u64) -> Result {
            let sender = ensure_signed(origin)?;

            <Value<T>>::insert(sender, value);

            Ok(())
        }
    }
}
