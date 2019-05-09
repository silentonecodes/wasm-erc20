#![cfg_attr(not(any(test, feature = "test-env")), no_std)]

use ink_core::{
    env::{
        self,
        AccountId,
        Balance,
    },
    memory::format,
    storage,
};
use ink_lang::contract;

contract! {
    /// @dev The storage items for a typical ERC20 token implementation.
    struct Erc20 {
        balances: storage::HashMap<AccountId, Balance>,
        allowances: storage::HashMap<(AccountId, AccountId), Balance>,
        total_supply: storage::Value<Balance>,
    }

    event Approval { owner: AccountId, spender: AccountId, value: Balance }
    event Transfer { from: Option<AccountId>, to: Option<AccountId>, value: Balance }

    impl Deploy for Erc20 {
        /// @dev Same as Constructor in Solidity
        fn deploy(&mut self, init_value: Balance) {
            env.println(&format!("Erc20::deploy(caller = {:?}, init_value = {:?})", env.caller(), init_value));
            self.total_supply.set(init_value);
            self.balances.insert(env.caller(), init_value);
            env.emit(Transfer{ from: None, to: Some(env.caller()), value: init_value });
        }
    }

    impl Erc20 {
        /// @dev Returns the total number of tokens in existence.
        pub(external) fn total_supply(&self) -> Balance {
            let total_supply = *self.total_supply;
            env.println(&format!("Erc20::total_supply = {:?}", total_supply));
            total_supply
        }

        /// @dev Returns the balance of the given address.
        /// @param owner The address to query the the balance of.
        pub(external) fn balance_of(&self, owner: AccountId) -> Balance {
            self.balance_of_or_zero(&owner)
        }

        /// @dev Returns the amount of tokens that an owner allowed to a spender.
        /// @param owner The address which owns the funds.
        /// @param spender The address which will spend the funds.
        pub(external) fn allowance(&self, owner: AccountId, spender: AccountId) -> Balance {
            self.allowance_or_zero(&owner, &spender)
        }

        /// @dev Transfers token from the sender to the `to` address.
        /// @param to The address which you want to transfer to.
        /// @param value the amount of tokens to be transferred
        pub(external) fn transfer(&mut self, to: AccountId, value: Balance) -> bool {
            env.println(&format!("Erc20::transfer(to = {:?}, value = {:?})", to, value));
            self.transfer_impl(env, env.caller(), to, value)
        }

        /// @dev Approve the passed address to spend the specified amount of tokens
        /// on the behalf of the message's sender.
        /// @param spender The address which will spend the funds.
        /// @param value The amount of tokens to be spent.
        pub(external) fn approve(&mut self, spender: AccountId, value: Balance) -> bool {
            env.println(&format!(
                "Erc20::approve(spender = {:?}, value = {:?})",
                spender, value
            ));
            let owner = env.caller();
            self.allowances.insert((owner, spender), value);
            env.emit(Approval{ owner, spender, value });
            true
        }

        /// @dev Transfer tokens from one address to another.
        /// @param from The address which you want to send tokens from.
        /// @param to The address which you want to transfer to.
        /// @param value the amount of tokens to be transferred.
        pub(external) fn transfer_from(&mut self, from: AccountId, to: AccountId, value: Balance) -> bool {
            env.println(&format!(
                "Erc20::transfer_from(from: {:?}, to = {:?}, value = {:?})",
                from, to, value
            ));
            self.transfer_impl(env, from, to, value)
        }
    }

    impl Erc20 {
        /// @dev Returns the allowance or 0 of there is no allowance.
        fn allowance_or_zero(&self, from: &AccountId, to: &AccountId) -> Balance {
            let allowance = self.allowances.get(&(*from, *to)).unwrap_or(&0);
            env::println(&format!(
                "Erc20::allowance_or_zero(from = {:?}, to = {:?}) = {:?}",
                from, to, allowance
            ));
            *allowance
        }

        /// @dev Returns the balance of the address or 0 if there is no balance.
        fn balance_of_or_zero(&self, of: &AccountId) -> Balance {
            let balance = self.balances.get(of).unwrap_or(&0);
            env::println(&format!(
                "Erc20::balance_of_or_zero(of = {:?}) = {:?}",
                of, balance
            ));
            *balance
        }

        /// @dev Transfers token from a specified address to another address.
        fn transfer_impl(
            &mut self,
            env: &ink_model::EnvHandler,
            from: AccountId,
            to: AccountId,
            value: Balance
        ) -> bool {
            let balance_from = self.balance_of_or_zero(&from);
            let balance_to = self.balance_of_or_zero(&to);
            if balance_from < value {
                return false
            }
            if !self.try_decrease_allowance(env, &from, value) {
                return false
            }
            self.balances.insert(from, balance_from - value);
            self.balances.insert(to, balance_to + value);
            env.emit(Transfer{ from: Some(from), to: Some(to), value });
            true
        }

        /// @dev Decreases the allowance and returns if it was successful.
        fn try_decrease_allowance(&mut self, env: &ink_model::EnvHandler, from: &AccountId, by: Balance) -> bool {
            // The owner of the coins doesn't need an allowance.
            if &env::caller() == from {
                return true
            }
            let caller = env.caller();
            let allowance = self.allowance_or_zero(from, &caller);
            if allowance < by {
                return false
            }
            self.allowances.insert((*from, caller), allowance - by);
            true
        }
    }
}




#[cfg(all(test, feature = "test-env"))]
mod tests {
    use super::*;
    use std::convert::TryFrom;

    #[test]
    fn deployment_works() {
        let alice = AccountId::try_from([0x0; 32]).unwrap();
        env::test::set_caller(alice);

        // Deploy the contract with some `init_value`
        let erc20 = Erc20::deploy_mock(1234);
        // Check that the `total_supply` is `init_value`
        assert_eq!(erc20.total_supply(), 1234);
        // Check that `balance_of` Alice is `init_value`
        assert_eq!(erc20.balance_of(alice), 1234);
    }

    #[test]
    fn transfer_works() {
        let alice = AccountId::try_from([0x0; 32]).unwrap();
        let bob = AccountId::try_from([0x1; 32]).unwrap();

        env::test::set_caller(alice);
        // Deploy the contract with some `init_value`
        let mut erc20 = Erc20::deploy_mock(1234);
        // Alice does not have enough funds for this
        assert_eq!(erc20.transfer(bob, 4321), false);
        // Alice can do this though
        assert_eq!(erc20.transfer(bob, 234), true);
        // Check Alice and Bob have the expected balance
        assert_eq!(erc20.balance_of(alice), 1000);
        assert_eq!(erc20.balance_of(bob), 234);
    }

    #[test]
    fn allowance_works() {
        let alice = AccountId::try_from([0x0; 32]).unwrap();
        let bob = AccountId::try_from([0x1; 32]).unwrap();
        let charlie = AccountId::try_from([0x2; 32]).unwrap();

        env::test::set_caller(alice);
        // Deploy the contract with some `init_value`
        let mut erc20 = Erc20::deploy_mock(1234);
        // Bob does not have an allowance from Alice's balance
        assert_eq!(erc20.allowance(alice, bob), 0);
        // Thus, Bob cannot transfer out of Alice's account
        env::test::set_caller(bob);
        assert_eq!(erc20.transfer_from(alice, bob, 1), false);
        // Alice can approve bob for some of her funds
        env::test::set_caller(alice);
        assert_eq!(erc20.approve(bob, 20), true);
        // And the allowance reflects that correctly
        assert_eq!(erc20.allowance(alice, bob), 20);

        // Charlie cannot send on behalf of Bob
        env::test::set_caller(charlie);
        assert_eq!(erc20.transfer_from(alice, bob, 10), false);
        // Bob cannot transfer more than he is allowed
        env::test::set_caller(bob);
        assert_eq!(erc20.transfer_from(alice, charlie, 25), false);
        // A smaller amount should work though
        assert_eq!(erc20.transfer_from(alice, charlie, 10), true);
        // Check that the allowance is updated
        assert_eq!(erc20.allowance(alice, bob), 10);
        // and the balance transferred to the right person
        assert_eq!(erc20.balance_of(charlie), 10);
    }

    #[test]
    fn events_work() {
        let alice = AccountId::try_from([0x0; 32]).unwrap();
        let bob = AccountId::try_from([0x1; 32]).unwrap();

        // No events to start
        env::test::set_caller(alice);
        assert_eq!(env::test::emitted_events().count(), 0);
        // Event should be emitted for initial minting
        let mut erc20 = Erc20::deploy_mock(1234);
        assert_eq!(env::test::emitted_events().count(), 1);
        // Event should be emitted for transfers
        assert_eq!(erc20.transfer(bob, 10), true);
        assert_eq!(env::test::emitted_events().count(), 2);
        // Event should be emitted for approvals
        assert_eq!(erc20.approve(bob, 20), true);
        assert_eq!(env::test::emitted_events().count(), 3);
    }
}

