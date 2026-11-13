use std::collections::{BTreeMap, HashMap};

use rust_decimal::Decimal;

use crate::models::{
    account::{Account, checked_add, checked_sub},
    transaction::{Transaction, TransactionType},
};

#[derive(Default)]
pub struct PaymentEngine {
    accounts: BTreeMap<u16, Account>,
    deposits: HashMap<u32, Deposit>,
}

struct Deposit {
    client: u16,
    amount: Decimal,
    state: DepositState,
}

#[derive(Clone, Copy)]
enum DepositState {
    Normal,
    Disputed,
    ChargedBack,
}

impl PaymentEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn process(&mut self, transaction: Transaction) {
        let account = self
            .accounts
            .entry(transaction.client)
            .or_insert_with(|| Account {
                id: transaction.client,
                ..Account::default()
            });

        if account.locked {
            return;
        }

        match transaction.transaction_type {
            TransactionType::Deposit => {
                if transaction.amount < Decimal::ZERO || self.deposits.contains_key(&transaction.tx)
                {
                    return;
                }

                if !update_balances(
                    checked_add(account.available, transaction.amount),
                    Some(account.held),
                    account,
                ) {
                    return;
                }

                self.deposits.insert(
                    transaction.tx,
                    Deposit {
                        client: transaction.client,
                        amount: transaction.amount,
                        state: DepositState::Normal,
                    },
                );
            }

            TransactionType::Withdrawal => {
                if transaction.amount < Decimal::ZERO || transaction.amount > account.available {
                    return;
                }

                update_balances(
                    checked_sub(account.available, transaction.amount),
                    Some(account.held),
                    account,
                );
            }

            TransactionType::Dispute | TransactionType::Resolve | TransactionType::Chargeback => {
                let Some(deposit) = self.deposits.get_mut(&transaction.tx) else {
                    return;
                };

                if deposit.client != transaction.client {
                    return;
                }

                match (transaction.transaction_type, deposit.state) {
                    (TransactionType::Dispute, DepositState::Normal) => {
                        if !update_balances(
                            checked_sub(account.available, deposit.amount),
                            checked_add(account.held, deposit.amount),
                            account,
                        ) {
                            return;
                        }

                        deposit.state = DepositState::Disputed;
                    }

                    (TransactionType::Resolve, DepositState::Disputed) => {
                        if !update_balances(
                            checked_add(account.available, deposit.amount),
                            checked_sub(account.held, deposit.amount),
                            account,
                        ) {
                            return;
                        }

                        deposit.state = DepositState::Normal;
                    }

                    (TransactionType::Chargeback, DepositState::Disputed) => {
                        if !update_balances(
                            Some(account.available),
                            checked_sub(account.held, deposit.amount),
                            account,
                        ) {
                            return;
                        }

                        deposit.state = DepositState::ChargedBack;
                        account.locked = true;
                    }

                    _ => {}
                }
            }
        }
    }

    pub fn accounts(&self) -> impl Iterator<Item = &Account> {
        self.accounts.values()
    }
}

fn update_balances(
    available: Option<Decimal>,
    held: Option<Decimal>,
    account: &mut Account,
) -> bool {
    let Some((available, held)) = available.zip(held) else {
        return false;
    };

    if checked_add(available, held).is_none() {
        return false;
    }

    account.available = available;
    account.held = held;

    true
}
