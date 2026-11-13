use rust_decimal::Decimal;
use rust_experiment::{
    models::{
        account::Account,
        transaction::{Transaction, TransactionType::*},
    },
    services::payment_engine::PaymentEngine,
};

fn apply(
    engine: &mut PaymentEngine,
    kind: rust_experiment::models::transaction::TransactionType,
    client: u16,
    tx: u32,
    amount: &str,
) {
    engine.process(Transaction {
        transaction_type: kind,
        client,
        tx,
        amount: amount.parse().unwrap(),
    })
}

fn account(engine: &PaymentEngine, client: u16) -> &Account {
    engine
        .accounts()
        .find(|account| account.id == client)
        .unwrap()
}

fn assert_balances(engine: &PaymentEngine, available: &str, held: &str, total: &str) {
    let account = account(engine, 1);
    assert_eq!(account.available, available.parse::<Decimal>().unwrap());
    assert_eq!(account.held, held.parse::<Decimal>().unwrap());
    assert_eq!(account.total(), Some(total.parse::<Decimal>().unwrap()));
}

fn disputed() -> PaymentEngine {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "100");
    apply(&mut engine, Dispute, 1, 1, "0");
    engine
}

#[test]
fn deposits_and_withdrawals_preserve_decimal_precision() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "1.0001");
    apply(&mut engine, Withdrawal, 1, 2, "0.5");
    assert_balances(&engine, "0.5001", "0", "0.5001");
    apply(&mut engine, Withdrawal, 1, 3, "0.5001");
    assert_balances(&engine, "0", "0", "0");
}

#[test]
fn withdrawal_cannot_spend_held_funds() {
    let mut engine = disputed();
    apply(&mut engine, Deposit, 1, 2, "10");
    apply(&mut engine, Withdrawal, 1, 3, "20");
    assert_balances(&engine, "10", "100", "110");
}

#[test]
fn negative_payments_are_ignored() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "10");
    apply(&mut engine, Deposit, 1, 2, "-5");
    apply(&mut engine, Withdrawal, 1, 3, "-5");
    assert_balances(&engine, "10", "0", "10");
}

#[test]
fn zero_payments_are_accepted() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "0");
    apply(&mut engine, Withdrawal, 1, 2, "0");
    assert_balances(&engine, "0", "0", "0");
}

#[test]
fn duplicate_deposit_does_not_credit_twice() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "10");
    apply(&mut engine, Deposit, 1, 1, "10");
    assert_balances(&engine, "10", "0", "10");
}

#[test]
fn dispute_holds_funds_without_changing_total() {
    let engine = disputed();
    assert_balances(&engine, "0", "100", "100");
}

#[test]
fn duplicate_dispute_does_not_hold_twice() {
    let mut engine = disputed();
    apply(&mut engine, Dispute, 1, 1, "0");
    assert_balances(&engine, "0", "100", "100");
}

#[test]
fn resolve_releases_held_funds() {
    let mut engine = disputed();
    apply(&mut engine, Resolve, 1, 1, "0");
    assert_balances(&engine, "100", "0", "100");
    assert!(!account(&engine, 1).locked);
}

#[test]
fn duplicate_resolve_does_not_release_twice() {
    let mut engine = disputed();
    apply(&mut engine, Resolve, 1, 1, "0");
    apply(&mut engine, Resolve, 1, 1, "0");
    assert_balances(&engine, "100", "0", "100");
}

#[test]
fn resolved_deposit_can_be_disputed_again() {
    let mut engine = disputed();
    apply(&mut engine, Resolve, 1, 1, "0");
    assert_balances(&engine, "100", "0", "100");
    apply(&mut engine, Dispute, 1, 1, "0");
    assert_balances(&engine, "0", "100", "100");
}

#[test]
fn chargeback_removes_only_held_funds_and_locks_account() {
    let mut engine = disputed();
    apply(&mut engine, Deposit, 1, 2, "50");
    apply(&mut engine, Chargeback, 1, 1, "0");
    assert_balances(&engine, "50", "0", "50");
    assert!(account(&engine, 1).locked);
}

#[test]
fn duplicate_chargeback_does_not_debit_twice() {
    let mut engine = disputed();
    apply(&mut engine, Chargeback, 1, 1, "0");
    apply(&mut engine, Chargeback, 1, 1, "0");
    assert_balances(&engine, "0", "0", "0");
}

#[test]
fn resolve_and_chargeback_require_an_active_dispute() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "100");
    for kind in [Resolve, Chargeback] {
        apply(&mut engine, kind, 1, 1, "0");
        assert_balances(&engine, "100", "0", "100");
        assert!(!account(&engine, 1).locked);
    }
}

#[test]
fn locked_accounts_ignore_every_transaction_type() {
    let mut engine = disputed();
    apply(&mut engine, Deposit, 1, 2, "50");
    apply(&mut engine, Chargeback, 1, 1, "0");
    let before = account(&engine, 1).clone();
    for (kind, tx, amount) in [
        (Deposit, 3, "10"),
        (Withdrawal, 4, "10"),
        (Dispute, 2, "0"),
        (Resolve, 1, "0"),
        (Chargeback, 1, "0"),
    ] {
        apply(&mut engine, kind, 1, tx, amount);
        assert_eq!(account(&engine, 1), &before);
    }
}

#[test]
fn wrong_client_cannot_modify_a_deposit() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, u32::MAX, "100");
    apply(&mut engine, Dispute, 2, u32::MAX, "0");
    assert_balances(&engine, "100", "0", "100");
    apply(&mut engine, Dispute, 1, u32::MAX, "0");
    for kind in [Resolve, Chargeback] {
        apply(&mut engine, kind, 2, u32::MAX, "0");
        assert_balances(&engine, "0", "100", "100");
        assert!(!account(&engine, 1).locked);
    }
    assert_eq!(account(&engine, 2).total(), Some(Decimal::ZERO));
}

#[test]
fn unknown_references_create_an_unchanged_empty_account() {
    let mut engine = PaymentEngine::new();
    for kind in [Dispute, Resolve, Chargeback] {
        apply(&mut engine, kind, 1, 99, "0");
    }
    assert_balances(&engine, "0", "0", "0");
    assert!(!account(&engine, 1).locked);
}

#[test]
fn withdrawals_cannot_be_disputed_or_charged_back() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "100");
    apply(&mut engine, Withdrawal, 1, 2, "40");
    for kind in [Dispute, Resolve, Chargeback] {
        apply(&mut engine, kind, 1, 2, "0");
        assert_balances(&engine, "60", "0", "60");
        assert!(!account(&engine, 1).locked);
    }
}

#[test]
fn failed_payments_cannot_be_disputed() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Withdrawal, 1, 1, "100");
    apply(&mut engine, Deposit, 1, 2, "-5");
    for tx in [1, 2] {
        apply(&mut engine, Dispute, 1, tx, "0");
    }
    assert_balances(&engine, "0", "0", "0");
}

#[test]
fn dispute_after_spending_can_make_available_negative() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "10");
    apply(&mut engine, Withdrawal, 1, 2, "8");
    apply(&mut engine, Dispute, 1, 1, "0");
    assert_balances(&engine, "-8", "10", "2");
}

#[test]
fn deposit_overflow_does_not_change_balances_or_store_the_deposit() {
    let mut engine = PaymentEngine::new();
    let max = Decimal::MAX.to_string();
    apply(&mut engine, Deposit, 1, 1, &max);
    apply(&mut engine, Deposit, 1, 2, "1");
    assert_balances(&engine, &max, "0", &max);
    apply(&mut engine, Withdrawal, 1, 3, "1");
    apply(&mut engine, Deposit, 1, 2, "1");
    assert_balances(&engine, &max, "0", &max);
}

#[test]
fn total_overflow_is_detected_before_available_is_changed() {
    let mut engine = PaymentEngine::new();
    let max = Decimal::MAX.to_string();
    apply(&mut engine, Deposit, 1, 1, &max);
    apply(&mut engine, Dispute, 1, 1, "0");
    apply(&mut engine, Deposit, 1, 2, "1");
    assert_balances(&engine, "0", &max, &max);
    apply(&mut engine, Resolve, 1, 1, "0");
    assert_balances(&engine, &max, "0", &max);
}

#[test]
fn dispute_overflow_does_not_change_balances_or_dispute_state() {
    let mut engine = PaymentEngine::new();
    let max = Decimal::MAX.to_string();
    apply(&mut engine, Deposit, 1, 1, &max);
    apply(&mut engine, Withdrawal, 1, 2, &max);
    apply(&mut engine, Deposit, 1, 3, &max);
    apply(&mut engine, Dispute, 1, 1, "0");
    apply(&mut engine, Dispute, 1, 3, "0");
    assert_balances(&engine, "0", &max, &max);
    apply(&mut engine, Resolve, 1, 1, "0");
    apply(&mut engine, Dispute, 1, 3, "0");
    assert_balances(&engine, "0", &max, &max);
}

#[test]
fn tiny_deposit_is_not_rounded_away_next_to_a_huge_balance() {
    let mut engine = PaymentEngine::new();
    let max = Decimal::MAX.to_string();
    apply(&mut engine, Deposit, 1, 1, &max);
    apply(&mut engine, Deposit, 1, 2, "0.0001");
    apply(&mut engine, Dispute, 1, 2, "0");
    assert_balances(&engine, &max, "0", &max);
    apply(&mut engine, Withdrawal, 1, 3, &max);
    apply(&mut engine, Deposit, 1, 2, "0.0001");
    assert_balances(&engine, "0.0001", "0", "0.0001");
}

#[test]
fn huge_deposit_cannot_erase_an_existing_small_balance() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "0.0001");
    apply(&mut engine, Deposit, 1, 2, &Decimal::MAX.to_string());
    assert_balances(&engine, "0.0001", "0", "0.0001");
}

#[test]
fn tiny_withdrawal_is_not_accepted_without_debiting_the_balance() {
    let mut engine = PaymentEngine::new();
    let max = Decimal::MAX.to_string();
    apply(&mut engine, Deposit, 1, 1, &max);
    apply(&mut engine, Withdrawal, 1, 2, "0.0001");
    assert_balances(&engine, &max, "0", &max);
}

#[test]
fn an_inexact_dispute_does_not_change_balances_or_deposit_state() {
    let mut engine = PaymentEngine::new();
    let max = Decimal::MAX.to_string();
    apply(&mut engine, Deposit, 1, 1, "0.0001");
    apply(&mut engine, Withdrawal, 1, 2, "0.0001");
    apply(&mut engine, Deposit, 1, 3, &max);
    apply(&mut engine, Dispute, 1, 1, "0");
    assert_balances(&engine, &max, "0", &max);
    apply(&mut engine, Withdrawal, 1, 4, &max);
    apply(&mut engine, Dispute, 1, 1, "0");
    assert_balances(&engine, "-0.0001", "0.0001", "0");
}

#[test]
fn total_precision_is_checked_before_accepting_a_deposit() {
    let mut engine = PaymentEngine::new();
    let max = Decimal::MAX.to_string();
    apply(&mut engine, Deposit, 1, 1, &max);
    apply(&mut engine, Dispute, 1, 1, "0");
    apply(&mut engine, Deposit, 1, 2, "0.0001");
    assert_balances(&engine, "0", &max, &max);
}

#[test]
fn separate_disputes_release_only_their_own_amounts() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "100");
    apply(&mut engine, Deposit, 1, 2, "50");
    apply(&mut engine, Dispute, 1, 1, "0");
    apply(&mut engine, Dispute, 1, 2, "0");
    assert_balances(&engine, "0", "150", "150");
    apply(&mut engine, Resolve, 1, 1, "0");
    assert_balances(&engine, "100", "50", "150");
    apply(&mut engine, Chargeback, 1, 2, "0");
    assert_balances(&engine, "100", "0", "100");
}

#[test]
fn frozen_account_keeps_other_pending_disputes_held() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "100");
    apply(&mut engine, Deposit, 1, 2, "50");
    apply(&mut engine, Dispute, 1, 1, "0");
    apply(&mut engine, Dispute, 1, 2, "0");
    apply(&mut engine, Chargeback, 1, 1, "0");
    apply(&mut engine, Resolve, 1, 2, "0");
    assert_balances(&engine, "0", "50", "50");
    assert!(account(&engine, 1).locked);
}

#[test]
fn chargeback_after_spending_can_leave_a_negative_total() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "10");
    apply(&mut engine, Withdrawal, 1, 2, "8");
    apply(&mut engine, Dispute, 1, 1, "0");
    apply(&mut engine, Chargeback, 1, 1, "0");
    assert_balances(&engine, "-8", "0", "-8");
    assert!(account(&engine, 1).locked);
}

#[test]
fn interleaved_clients_and_unordered_ids_remain_independent() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, u16::MAX, u32::MAX, "5");
    apply(&mut engine, Deposit, 0, 0, "2");
    apply(&mut engine, Withdrawal, u16::MAX, 5, "1");
    apply(&mut engine, Dispute, 0, 0, "0");
    let ids: Vec<_> = engine.accounts().map(|account| account.id).collect();
    assert_eq!(ids, vec![0, u16::MAX]);
    assert_eq!(account(&engine, 0).available, Decimal::ZERO);
    assert_eq!(account(&engine, 0).held, Decimal::from(2));
    assert_eq!(account(&engine, u16::MAX).available, Decimal::from(4));
}

#[test]
fn duplicate_deposit_id_cannot_be_reassigned_to_another_client() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "10");
    apply(&mut engine, Deposit, 2, 1, "99");
    apply(&mut engine, Dispute, 1, 1, "0");
    assert_balances(&engine, "0", "10", "10");
    assert_eq!(account(&engine, 2).total(), Some(Decimal::ZERO));
}

#[test]
fn dispute_events_use_the_original_deposit_amount() {
    let mut engine = PaymentEngine::new();
    apply(&mut engine, Deposit, 1, 1, "10");
    apply(&mut engine, Dispute, 1, 1, "999");
    assert_balances(&engine, "0", "10", "10");
    apply(&mut engine, Resolve, 1, 1, "999");
    assert_balances(&engine, "10", "0", "10");
}
