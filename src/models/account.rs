use rust_decimal::Decimal;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

impl Account {
    pub fn total(&self) -> Option<Decimal> {
        self.available.checked_add(self.held)
    }
}
