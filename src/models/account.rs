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

pub(crate) fn checked_add(left: Decimal, right: Decimal) -> Option<Decimal> {
    let mut scale = left.scale().max(right.scale());
    let left = left
        .mantissa()
        .checked_mul(10_i128.checked_pow(scale - left.scale())?)?;
    let right = right
        .mantissa()
        .checked_mul(10_i128.checked_pow(scale - right.scale())?)?;
    let mut sum = left.checked_add(right)?;

    while scale > 0 && sum % 10 == 0 {
        sum /= 10;
        scale -= 1;
    }
    Decimal::try_from_i128_with_scale(sum, scale).ok()
}

pub(crate) fn checked_sub(left: Decimal, right: Decimal) -> Option<Decimal> {
    checked_add(left, -right)
}