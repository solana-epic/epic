/// Recursively strips common Solana and Rust wrapper types (Box, Option, Account, etc.)
/// to extract the core user-defined state structure name.
pub fn unpack_nested_generics(raw_type: &str) -> String {
    let mut current = raw_type.trim().to_string();
    loop {
        if current.starts_with("Box<") && current.ends_with('>') {
            current = current[4..current.len() - 1].trim().to_string();
        } else if current.starts_with("Option<") && current.ends_with('>') {
            current = current[7..current.len() - 1].trim().to_string();
        } else if (current.starts_with("Account<")
            || current.starts_with("AccountLoader<")
            || current.starts_with("InterfaceAccount<"))
            && current.ends_with('>')
        {
            if let Some(comma_idx) = current.find(',') {
                current = current[comma_idx + 1..current.len() - 1].trim().to_string();
            } else {
                break;
            }
        } else {
            break;
        }
    }
    current
}
