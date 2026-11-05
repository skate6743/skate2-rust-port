//! TU3 ExpressionOperator::GetActivationMasked (82C12508).
//! Excluded children do not participate in AND/OR/NOT. Preserve lazy calls:
//! condition operations may read or modify their own instance state.

/// ParseOperator 82C12438: case-sensitive strings, zero for missing/unknown.
pub fn parse_operator(name: Option<&str>) -> u32 {
    match name {
        Some("and") => 1,
        Some("or") => 2,
        Some("not") => 3,
        _ => 0,
    }
}

/// Execute the complete operator with the native virtual child-call boundary.
/// The callback receives its ordered child index and the initially true
/// excluded byte; it returns the native result word. Context/mask are supplied
/// by the caller's closure, without an invented default condition result.
/// NOT requires a first child, as in the native caller contract.
pub fn evaluate_operator(
    operator: u32,
    child_count: usize,
    excluded: &mut u8,
    mut child: impl FnMut(usize, &mut u8) -> u32,
) -> u32 {
    if operator > 3 {
        return 1; // Native default leaves the caller's excluded byte untouched.
    }
    if operator == 3 {
        assert!(child_count > 0, "stock NOT expression requires a child");
        let mut child_excluded = 1;
        let value = child(0, &mut child_excluded) as u8;
        *excluded = child_excluded;
        return if child_excluded != 0 {
            1
        } else {
            u32::from(value == 0)
        };
    }
    let is_or = operator == 2;
    let mut value = if is_or { 0_u32 } else { 1_u32 };
    let mut all_excluded = 1;
    for index in 0..child_count {
        if (is_or && value as u8 != 0) || (!is_or && value as u8 == 0) {
            break;
        }
        let mut child_excluded = 1;
        let result = child(index, &mut child_excluded);
        if child_excluded == 0 {
            all_excluded = 0;
            value = if is_or {
                result
            } else {
                u32::from((result as u8) & (value as u8))
            };
        }
    }
    *excluded = all_excluded;
    if all_excluded != 0 { 1 } else { value }
}
