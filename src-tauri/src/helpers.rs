// helpers.rs

pub fn is_valid_xorname(input: &str) -> bool {
    if let Ok(decoded) = hex::decode(input) {
        decoded.len() == 32
    } else {
        false
    }
}
