// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Vector session token types for Cosmos DB.

use std::{collections::HashMap, fmt, str::FromStr};

use super::Error;
use crate::{Lsn, RegionId};

/// A vector session token for Cosmos DB operations.
///
/// Vector session tokens are used to maintain consistency across operations
/// in Cosmos DB by tracking logical sequence numbers (LSNs) at both global
/// and regional levels.
#[derive(Debug, Clone, PartialEq)]
pub struct VectorSessionToken {
    /// The version of the session token format.
    pub version: u64,

    /// The global logical sequence number.
    pub global_lsn: Lsn,

    /// A mapping of region IDs to their respective logical sequence numbers.
    pub regional_lsns: HashMap<RegionId, Lsn>,
}

fn parse_u64_from_slice(s: &str) -> Result<u64, ()> {
    if s.is_empty() {
        return Err(());
    }

    let mut result = 0u64;
    for byte in s.bytes() {
        let digit = match byte {
            b'0'..=b'9' => (byte - b'0') as u64,
            _ => return Err(()),
        };

        result = result.checked_mul(10).ok_or(())?;
        result = result.checked_add(digit).ok_or(())?;
    }

    Ok(result)
}

fn parse_u32_from_slice(s: &str) -> Result<u32, ()> {
    if s.is_empty() {
        return Err(());
    }

    let mut result = 0u32;
    for byte in s.bytes() {
        let digit = match byte {
            b'0'..=b'9' => (byte - b'0') as u32,
            _ => return Err(()),
        };

        result = result.checked_mul(10).ok_or(())?;
        result = result.checked_add(digit).ok_or(())?;
    }

    Ok(result)
}

impl FromStr for VectorSessionToken {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(Error::EmptyInput);
        }

        let mut chars = s.char_indices();

        // Find first '#' delimiter
        let version_end = loop {
            match chars.next() {
                Some((i, '#')) => break i,
                Some((_, _)) => continue,
                None => return Err(Error::MissingComponents),
            }
        };

        let version_str = &s[..version_end];
        let version = parse_u64_from_slice(version_str)
            .map_err(|_| Error::InvalidVersion(version_str.to_string()))?;

        // Find second '#' delimiter or end of string
        let global_lsn_start = version_end + 1;
        let global_lsn_end = loop {
            match chars.next() {
                Some((i, '#')) => break i,
                Some((_, _)) => continue,
                None => break s.len(),
            }
        };

        if global_lsn_start >= s.len() {
            return Err(Error::MissingComponents);
        }

        let global_lsn_str = &s[global_lsn_start..global_lsn_end];
        let global_lsn_value = parse_u64_from_slice(global_lsn_str)
            .map_err(|_| Error::InvalidGlobalLsn(global_lsn_str.to_string()))?;
        let global_lsn = Lsn::new(global_lsn_value);

        let mut regional_lsns = HashMap::new();

        // Parse regional components if any
        if global_lsn_end < s.len() {
            let mut current_pos = global_lsn_end + 1;

            while current_pos < s.len() {
                // Find next '#' or end of string
                let component_end = s[current_pos..]
                    .find('#')
                    .map(|pos| current_pos + pos)
                    .unwrap_or(s.len());

                let component = &s[current_pos..component_end];

                // Find '=' in component
                let equals_pos = component
                    .find('=')
                    .ok_or_else(|| Error::MalformedRegionalComponent(component.to_string()))?;

                let region_id_str = &component[..equals_pos];
                let region_lsn_str = &component[equals_pos + 1..];

                if region_id_str.is_empty() || region_lsn_str.is_empty() {
                    return Err(Error::MalformedRegionalComponent(component.to_string()));
                }

                let region_id = parse_u32_from_slice(region_id_str)
                    .map_err(|_| Error::InvalidRegionId(region_id_str.to_string()))?;
                let region_lsn = parse_u64_from_slice(region_lsn_str)
                    .map_err(|_| Error::InvalidRegionLsn(region_lsn_str.to_string()))?;

                regional_lsns.insert(RegionId::new(region_id), Lsn::new(region_lsn));

                current_pos = component_end + 1;
            }
        }

        Ok(VectorSessionToken {
            version,
            global_lsn,
            regional_lsns,
        })
    }
}

impl VectorSessionToken {
    /// Merges this token with another token to create a new token representing
    /// the highest progress from both tokens. This operation is commutative.
    pub fn merge(self, other: VectorSessionToken) -> Result<VectorSessionToken, Error> {
        // Determine which token has the higher version
        let (higher_version_token, lower_version_token) = if self.version >= other.version {
            (self, other)
        } else {
            (other, self)
        };

        // If versions are the same, validate that regions are compatible
        if higher_version_token.version == lower_version_token.version {
            // For same version, tokens must have the same set of regions
            let self_regions: std::collections::HashSet<_> =
                higher_version_token.regional_lsns.keys().collect();
            let other_regions: std::collections::HashSet<_> =
                lower_version_token.regional_lsns.keys().collect();

            if self_regions != other_regions {
                return Err(Error::TokensCannotBeMerged(
                    "tokens have same version but different regions".to_string(),
                ));
            }

            // Merge by taking maximum values for each component
            let mut merged_regional_lsns = std::collections::HashMap::new();
            for (region_id, &self_lsn) in &higher_version_token.regional_lsns {
                let other_lsn = lower_version_token.regional_lsns[region_id];
                merged_regional_lsns.insert(*region_id, std::cmp::max(self_lsn, other_lsn));
            }

            return Ok(VectorSessionToken {
                version: higher_version_token.version,
                global_lsn: std::cmp::max(
                    higher_version_token.global_lsn,
                    lower_version_token.global_lsn,
                ),
                regional_lsns: merged_regional_lsns,
            });
        }

        // Different versions: use the higher version token as base
        // and merge regional LSNs where regions exist in both
        let mut merged_regional_lsns = higher_version_token.regional_lsns.clone();

        for (region_id, &lower_lsn) in &lower_version_token.regional_lsns {
            if let Some(&higher_lsn) = merged_regional_lsns.get(region_id) {
                merged_regional_lsns.insert(*region_id, std::cmp::max(higher_lsn, lower_lsn));
            }
        }

        Ok(VectorSessionToken {
            version: higher_version_token.version,
            global_lsn: higher_version_token.global_lsn,
            regional_lsns: merged_regional_lsns,
        })
    }
}

impl fmt::Display for VectorSessionToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}#{}", self.version, self.global_lsn.value())?;

        for (region_id, region_lsn) in self.regional_lsns.iter() {
            write!(f, "#{}={}", region_id.value(), region_lsn.value())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_token() {
        let token_str = "1#1000";
        let token: VectorSessionToken = token_str.parse().unwrap();

        assert_eq!(token.version, 1);
        assert_eq!(token.global_lsn, Lsn::new(1000));
        assert!(token.regional_lsns.is_empty());
    }

    #[test]
    fn parse_token_with_single_region() {
        let token_str = "2#2000#100=1500";
        let token: VectorSessionToken = token_str.parse().unwrap();

        assert_eq!(token.version, 2);
        assert_eq!(token.global_lsn, Lsn::new(2000));
        assert_eq!(token.regional_lsns.len(), 1);
        assert_eq!(token.regional_lsns[&RegionId::new(100)], Lsn::new(1500));
    }

    #[test]
    fn parse_token_with_multiple_regions() {
        let token_str = "3#3000#100=1500#200=2500#300=3500";
        let token: VectorSessionToken = token_str.parse().unwrap();

        assert_eq!(token.version, 3);
        assert_eq!(token.global_lsn, Lsn::new(3000));
        assert_eq!(token.regional_lsns.len(), 3);
        assert_eq!(token.regional_lsns[&RegionId::new(100)], Lsn::new(1500));
        assert_eq!(token.regional_lsns[&RegionId::new(200)], Lsn::new(2500));
        assert_eq!(token.regional_lsns[&RegionId::new(300)], Lsn::new(3500));
    }

    #[test]
    fn parse_empty_string_fails() {
        let result: Result<VectorSessionToken, _> = "".parse();
        assert_eq!(result.unwrap_err(), Error::EmptyInput);
    }

    #[test]
    fn parse_missing_global_lsn_fails() {
        let result: Result<VectorSessionToken, _> = "1".parse();
        assert_eq!(result.unwrap_err(), Error::MissingComponents);
    }

    #[test]
    fn parse_missing_version_fails() {
        let result: Result<VectorSessionToken, _> = "#1000".parse();
        assert_eq!(result.unwrap_err(), Error::InvalidVersion("".to_string()));
    }

    #[test]
    fn parse_invalid_version_fails() {
        let result: Result<VectorSessionToken, _> = "not_a_number#1000".parse();
        assert_eq!(
            result.unwrap_err(),
            Error::InvalidVersion("not_a_number".to_string())
        );
    }

    #[test]
    fn parse_invalid_global_lsn_fails() {
        let result: Result<VectorSessionToken, _> = "1#not_a_number".parse();
        assert_eq!(
            result.unwrap_err(),
            Error::InvalidGlobalLsn("not_a_number".to_string())
        );
    }

    #[test]
    fn parse_invalid_region_id_fails() {
        let result: Result<VectorSessionToken, _> = "1#1000#not_a_number=1500".parse();
        assert_eq!(
            result.unwrap_err(),
            Error::InvalidRegionId("not_a_number".to_string())
        );
    }

    #[test]
    fn parse_invalid_region_lsn_fails() {
        let result: Result<VectorSessionToken, _> = "1#1000#100=not_a_number".parse();
        assert_eq!(
            result.unwrap_err(),
            Error::InvalidRegionLsn("not_a_number".to_string())
        );
    }

    #[test]
    fn parse_malformed_region_pair_fails() {
        let result: Result<VectorSessionToken, _> = "1#1000#100".parse();
        assert_eq!(
            result.unwrap_err(),
            Error::MalformedRegionalComponent("100".to_string())
        );

        let result: Result<VectorSessionToken, _> = "1#1000#100=".parse();
        assert_eq!(
            result.unwrap_err(),
            Error::MalformedRegionalComponent("100=".to_string())
        );

        let result: Result<VectorSessionToken, _> = "1#1000#=1500".parse();
        assert_eq!(
            result.unwrap_err(),
            Error::MalformedRegionalComponent("=1500".to_string())
        );
    }

    #[test]
    fn parse_version_overflow_fails() {
        let result: Result<VectorSessionToken, _> = "18446744073709551616#1000".parse();
        assert_eq!(
            result.unwrap_err(),
            Error::InvalidVersion("18446744073709551616".to_string())
        );
    }

    #[test]
    fn parse_global_lsn_overflow_fails() {
        let result: Result<VectorSessionToken, _> = "1#18446744073709551616".parse();
        assert_eq!(
            result.unwrap_err(),
            Error::InvalidGlobalLsn("18446744073709551616".to_string())
        );
    }

    #[test]
    fn parse_region_id_overflow_fails() {
        let result: Result<VectorSessionToken, _> = "1#1000#4294967296=1500".parse();
        assert_eq!(
            result.unwrap_err(),
            Error::InvalidRegionId("4294967296".to_string())
        );
    }

    #[test]
    fn parse_region_lsn_overflow_fails() {
        let result: Result<VectorSessionToken, _> = "1#1000#100=18446744073709551616".parse();
        assert_eq!(
            result.unwrap_err(),
            Error::InvalidRegionLsn("18446744073709551616".to_string())
        );
    }

    #[test]
    fn parse_duplicate_region_ids() {
        let token_str = "1#1000#100=1500#100=2500";
        let token: VectorSessionToken = token_str.parse().unwrap();

        assert_eq!(token.version, 1);
        assert_eq!(token.global_lsn, Lsn::new(1000));
        assert_eq!(token.regional_lsns.len(), 1);
        assert_eq!(token.regional_lsns[&RegionId::new(100)], Lsn::new(2500));
    }

    #[test]
    fn display_minimal_token() {
        let token = VectorSessionToken {
            version: 1,
            global_lsn: Lsn::new(1000),
            regional_lsns: HashMap::new(),
        };

        assert_eq!(token.to_string(), "1#1000");
    }

    #[test]
    fn display_token_with_regions() {
        let mut regional_lsns = HashMap::new();
        regional_lsns.insert(RegionId::new(100), Lsn::new(1500));
        regional_lsns.insert(RegionId::new(200), Lsn::new(2500));

        let token = VectorSessionToken {
            version: 2,
            global_lsn: Lsn::new(2000),
            regional_lsns,
        };

        let result = token.to_string();
        assert!(result.starts_with("2#2000"));
        assert!(result.contains("100=1500"));
        assert!(result.contains("200=2500"));
    }

    #[test]
    fn roundtrip_parsing() {
        let original = "3#3000#100=1500#200=2500";
        let token: VectorSessionToken = original.parse().unwrap();
        let regenerated = token.to_string();
        let reparsed: VectorSessionToken = regenerated.parse().unwrap();

        assert_eq!(token, reparsed);
    }

    #[test]
    fn can_advance_to_higher_version_can_advance_to() {
        let current: VectorSessionToken = "1#1000".parse().unwrap();
        let other: VectorSessionToken = "2#500".parse().unwrap();

        assert!(current.can_advance_to(&other).unwrap());
    }

    #[test]
    fn can_advance_to_same_version_higher_global_lsn_can_advance_to() {
        let current: VectorSessionToken = "1#1000".parse().unwrap();
        let other: VectorSessionToken = "1#2000".parse().unwrap();

        assert!(current.can_advance_to(&other).unwrap());
    }

    #[test]
    fn can_advance_to_same_version_lower_global_lsn_is_invalid() {
        let current: VectorSessionToken = "1#2000".parse().unwrap();
        let other: VectorSessionToken = "1#1000".parse().unwrap();

        assert!(!current.can_advance_to(&other).unwrap());
    }

    #[test]
    fn cannot_advance_to_lower_version() {
        let current: VectorSessionToken = "2#1000".parse().unwrap();
        let other: VectorSessionToken = "1#2000".parse().unwrap();

        assert!(!current.can_advance_to(&other).unwrap());
    }

    #[test]
    fn can_advance_to_regional_lsn_progression() {
        let current: VectorSessionToken = "1#1000#100=500".parse().unwrap();
        let other: VectorSessionToken = "1#1000#100=1000".parse().unwrap();

        assert!(current.can_advance_to(&other).unwrap());
    }

    #[test]
    fn cannot_advance_to_regional_lsn_regression() {
        let current: VectorSessionToken = "1#1000#100=1000".parse().unwrap();
        let other: VectorSessionToken = "1#1000#100=500".parse().unwrap();

        assert!(!current.can_advance_to(&other).unwrap());
    }

    #[test]
    fn can_advance_to_same_version_different_region_count_fails() {
        let current: VectorSessionToken = "1#1000#100=500".parse().unwrap();
        let other: VectorSessionToken = "1#1000#100=500#200=600".parse().unwrap();

        let result = current.can_advance_to(&other);
        assert!(result.is_err());

        assert!(matches!(result.unwrap_err(), Error::InvalidRegions { .. }));
    }

    #[test]
    fn can_advance_to_same_version_missing_region_in_current_fails() {
        let current: VectorSessionToken = "1#1000#100=500".parse().unwrap();
        let other: VectorSessionToken = "1#1000#100=500#200=600".parse().unwrap();

        let result = current.can_advance_to(&other);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidRegions { .. }));
    }

    #[test]
    fn can_advance_to_same_version_missing_region_in_other_fails() {
        let current: VectorSessionToken = "1#1000#100=500#200=600".parse().unwrap();
        let other: VectorSessionToken = "1#1000#100=500".parse().unwrap();

        let result = current.can_advance_to(&other);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::InvalidRegions { .. }));
    }

    #[test]
    fn can_advance_to_different_version_missing_region_is_allowed() {
        let current: VectorSessionToken = "1#1000#100=500".parse().unwrap();
        let other: VectorSessionToken = "2#1000#100=500#200=600".parse().unwrap();

        // When other has higher version, missing regions in current are ignored
        assert!(current.can_advance_to(&other).unwrap());
    }

    #[test]
    fn merge_same_version_takes_max_values() {
        let token1: VectorSessionToken = "2#1000#100=500#200=600".parse().unwrap();
        let token2: VectorSessionToken = "2#1200#100=800#200=400".parse().unwrap();

        let merged = token1.merge(token2).unwrap();

        assert_eq!(merged.version, 2);
        assert_eq!(merged.global_lsn, Lsn::new(1200));
        assert_eq!(merged.regional_lsns[&RegionId::new(100)], Lsn::new(800));
        assert_eq!(merged.regional_lsns[&RegionId::new(200)], Lsn::new(600));
    }

    #[test]
    fn merge_different_versions_takes_higher_version() {
        let token1: VectorSessionToken = "1#2000#100=1000".parse().unwrap();
        let token2: VectorSessionToken = "2#1000#100=500".parse().unwrap();

        let merged = token1.merge(token2).unwrap();

        assert_eq!(merged.version, 2);
        assert_eq!(merged.global_lsn, Lsn::new(1000)); // From higher version token
        assert_eq!(merged.regional_lsns[&RegionId::new(100)], Lsn::new(1000)); // Max of both
    }

    #[test]
    fn merge_is_commutative() {
        let token1: VectorSessionToken = "2#1000#100=500#200=600".parse().unwrap();
        let token2: VectorSessionToken = "2#1200#100=800#200=400".parse().unwrap();

        let merged1 = token1.clone().merge(token2.clone()).unwrap();
        let merged2 = token2.merge(token1).unwrap();

        assert_eq!(merged1, merged2);
    }

    #[test]
    fn merge_no_regions() {
        let token1: VectorSessionToken = "1#1000".parse().unwrap();
        let token2: VectorSessionToken = "1#1200".parse().unwrap();

        let merged = token1.merge(token2).unwrap();

        assert_eq!(merged.version, 1);
        assert_eq!(merged.global_lsn, Lsn::new(1200));
        assert!(merged.regional_lsns.is_empty());
    }

    #[test]
    fn merge_one_token_dominates() {
        let token1: VectorSessionToken = "2#2000#100=1000#200=800".parse().unwrap();
        let token2: VectorSessionToken = "2#1000#100=500#200=600".parse().unwrap();

        let merged = token1.clone().merge(token2).unwrap();

        // Should return token1 since it dominates in all aspects
        assert_eq!(merged, token1);
    }

    #[test]
    fn merge_different_version_missing_regions_allowed() {
        let token1: VectorSessionToken = "1#1000#100=500".parse().unwrap();
        let token2: VectorSessionToken = "2#1200#100=800#200=600".parse().unwrap();

        let merged = token1.merge(token2).unwrap();

        assert_eq!(merged.version, 2);
        assert_eq!(merged.global_lsn, Lsn::new(1200)); // From higher version
        assert_eq!(merged.regional_lsns.len(), 2); // Only regions from higher version
        assert_eq!(merged.regional_lsns[&RegionId::new(100)], Lsn::new(800)); // Max of both
        assert_eq!(merged.regional_lsns[&RegionId::new(200)], Lsn::new(600)); // Only in higher version
    }

    #[test]
    fn merge_same_version_incompatible_regions_fails() {
        let token1: VectorSessionToken = "2#1000#100=500".parse().unwrap();
        let token2: VectorSessionToken = "2#1200#200=600".parse().unwrap();

        let result = token1.merge(token2);

        assert!(result.is_err());
        match result.unwrap_err() {
            Error::TokensCannotBeMerged(reason) => {
                assert!(reason.contains("same version"));
                assert!(reason.contains("different regions"));
            }
            other => panic!("Expected TokensCannotBeMerged error, got: {:?}", other),
        }
    }
}
