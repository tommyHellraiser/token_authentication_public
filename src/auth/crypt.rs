use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use error_mapper::TheResult;
use rand::{Rng, thread_rng};
use rand::distributions::{Distribution};

struct TokenCharset;

impl Distribution<u8> for TokenCharset {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> u8 {
        const RANGE: u32 = 26 + 26 + 36;
        // Same situation as with the password hasher, I won't publish my personal CHARSET for
        // security reasons, even though the string is randomized
        const TOKEN_CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
        loop {
            let var = rng.next_u32() >> (32 - 6);
            if var < RANGE {
                return TOKEN_CHARSET[var as usize];
            }
        }
    }
}

pub fn generate_session_token() -> TheResult<String> {
    //generate random characters in a string
    Ok(
        thread_rng()
            .sample_iter(&TokenCharset)
            .take(40)
            .map(char::from).collect::<String>()
    )
}

pub fn generate_hash(string: &str) -> String {

    let mut hasher = DefaultHasher::new();
    string.hash(&mut hasher);
    hasher.finish().to_string()

}
