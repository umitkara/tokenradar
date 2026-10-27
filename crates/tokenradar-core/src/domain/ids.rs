//! Row ids as newtypes, so a `SessionId` can never be passed where a `ModelId` is expected.

use serde::{Deserialize, Serialize};
use specta::Type;

macro_rules! row_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Type,
        )]
        #[serde(transparent)]
        pub struct $name(pub i64);

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

row_id!(
    /// Row id in `accounts`.
    AccountId
);
row_id!(
    /// Row id in `sessions`.
    SessionId
);
row_id!(
    /// Row id in `projects`.
    ProjectId
);
row_id!(
    /// Row id in `models`.
    ModelId
);
