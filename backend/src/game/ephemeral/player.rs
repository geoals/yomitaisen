pub struct EphemeralPlayer {
    pub id: String,
    pub display_name: String,
}

impl EphemeralPlayer {
    pub fn new(display_name: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            display_name: display_name.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_player_has_id_and_display_name() {
        let player = EphemeralPlayer::new("Alice");
        assert!(!player.id.is_empty());
        assert_eq!(player.display_name, "Alice");
    }

    #[test]
    fn each_player_gets_unique_id() {
        let p1 = EphemeralPlayer::new("Alice");
        let p2 = EphemeralPlayer::new("Bob");
        assert_ne!(p1.id, p2.id);
    }
}
