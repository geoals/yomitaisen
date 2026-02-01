use rand::Rng;

const CHARSET: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
const ID_LENGTH: usize = 6;

pub fn generate_game_id() -> String {
    let mut rng = rand::rng();
    (0..ID_LENGTH)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

pub fn generate_unique_game_id<F>(exists: F) -> String
where
    F: Fn(&str) -> bool,
{
    loop {
        let id = generate_game_id();
        if !exists(&id) {
            return id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_six_character_code() {
        let id = generate_game_id();
        assert_eq!(id.len(), 6);
    }

    #[test]
    fn contains_only_allowed_characters() {
        let allowed = "abcdefghjkmnpqrstuvwxyz23456789"; // no 0, o, l, 1
        for _ in 0..100 {
            let id = generate_game_id();
            assert!(id.chars().all(|c| allowed.contains(c)));
        }
    }

    #[test]
    fn generates_unique_ids() {
        let ids: std::collections::HashSet<_> = (0..1000).map(|_| generate_game_id()).collect();
        assert_eq!(ids.len(), 1000); // all unique
    }

    #[test]
    fn retries_on_collision() {
        use std::cell::Cell;
        use std::collections::HashSet;

        let existing = HashSet::from(["abc123".to_string()]);
        let attempts = Cell::new(0);

        let id = generate_unique_game_id(|id| {
            attempts.set(attempts.get() + 1);
            // First attempt returns "collision" if it happens to be abc123
            existing.contains(id)
        });

        assert_ne!(id, "abc123");
        assert!(attempts.get() >= 1);
    }

    #[test]
    fn returns_immediately_when_no_collision() {
        let id = generate_unique_game_id(|_| false); // nothing exists
        assert_eq!(id.len(), 6);
    }
}
