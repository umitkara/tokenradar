//! Monthly plan fees for the plan value card (`pricing/plans.toml`, Step 14b).

use std::sync::LazyLock;

use serde::Deserialize;

use crate::domain::MicroUsd;

#[derive(Debug, Deserialize)]
struct Table {
    schema: u32,
    plan: Vec<Row>,
}

#[derive(Debug, Deserialize)]
struct Row {
    name: String,
    subscription: String,
    tier: Option<String>,
    monthly_usd: f64,
}

static TABLE: LazyLock<Table> = LazyLock::new(|| {
    let table: Table = toml::from_str(include_str!("../pricing/plans.toml"))
        .expect("embedded plans.toml parses (checked by tests)");
    assert_eq!(table.schema, 1, "plans.toml schema");
    table
});

/// A plan's name and monthly fee from the embedded table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFee {
    pub name: String,
    pub monthly: MicroUsd,
}

/// The fee for `.credentials.json` `subscriptionType` / `rateLimitTier`; `None` for a plan
/// the table does not price (Team, Enterprise, unknown).
pub fn plan_fee(subscription: Option<&str>, tier: Option<&str>) -> Option<PlanFee> {
    let sub = subscription?.to_ascii_lowercase();
    let tier = tier.unwrap_or_default().to_ascii_lowercase();
    TABLE
        .plan
        .iter()
        .find(|r| r.subscription == sub && r.tier.as_deref().is_none_or(|t| tier.contains(t)))
        .map(|r| PlanFee {
            name: r.name.clone(),
            monthly: usd(r.monthly_usd),
        })
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn usd(v: f64) -> MicroUsd {
    MicroUsd((v * 1_000_000.0).round() as i64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_plans_and_tiers() {
        let fee = |s, t| plan_fee(s, t).map(|p| (p.name, p.monthly.0));
        assert_eq!(
            fee(Some("max"), Some("default_claude_max_20x")),
            Some(("Ultra".into(), 200_000_000))
        );
        assert_eq!(
            fee(Some("max"), Some("default_claude_max_5x")),
            Some(("Max".into(), 100_000_000))
        );
        assert_eq!(fee(Some("Pro"), None), Some(("Pro".into(), 20_000_000)));
        assert_eq!(fee(Some("team"), None), None);
        assert_eq!(fee(None, None), None);
    }
}
