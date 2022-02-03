use std::fmt::Display;

pub mod config;
mod error;
pub mod session;

pub use error::*;

pub fn get_sign_in_key<D: Display>(user_id: D) -> String {
    format!("user:sign_in:{}", user_id)
}

#[cfg(test)]
mod tests {
    use super::get_sign_in_key;
    use uuid::Uuid;

    #[test]
    fn simple() {
        let uuid = Uuid::new_v4().simple();
        assert_eq!(get_sign_in_key(&uuid), get_sign_in_key(&uuid));
    }
}
