mod common;

use common::*;
use futures_util::SinkExt;
use yomitaisen::messages::ServerMessage;

#[tokio::test]
async fn create_game_returns_game_id_and_waits() {
    let server = spawn_test_server().await;
    let mut ws = connect_ephemeral(&server).await;

    ws.send(create_game_msg("Alice")).await.unwrap();

    // Should receive GameCreated with a game_id
    let msg = recv(&mut ws).await;
    let game_id = match msg {
        ServerMessage::GameCreated { game_id } => {
            assert_eq!(game_id.len(), 6); // Short code
            game_id
        }
        other => panic!("Expected GameCreated, got {:?}", other),
    };

    // Should then receive WaitingForOpponent
    assert_eq!(recv(&mut ws).await, ServerMessage::WaitingForOpponent);

    // Game ID should be reusable for joining
    assert!(!game_id.is_empty());
}

#[tokio::test]
async fn join_game_starts_match() {
    let server = spawn_test_server().await;

    // Host creates game
    let mut host_ws = connect_ephemeral(&server).await;
    host_ws.send(create_game_msg("Alice")).await.unwrap();

    let game_id = match recv(&mut host_ws).await {
        ServerMessage::GameCreated { game_id } => game_id,
        other => panic!("Expected GameCreated, got {:?}", other),
    };
    assert_eq!(recv(&mut host_ws).await, ServerMessage::WaitingForOpponent);

    // Guest joins with game ID
    let mut guest_ws = connect_ephemeral(&server).await;
    guest_ws.send(join_game_msg(&game_id, "Bob")).await.unwrap();

    // Host receives OpponentJoined then GameStart
    assert!(matches!(
        recv(&mut host_ws).await,
        ServerMessage::OpponentJoined { opponent_name } if opponent_name == "Bob"
    ));
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::GameStart { .. }));

    // Guest receives GameStart
    assert!(matches!(recv(&mut guest_ws).await, ServerMessage::GameStart { .. }));

    // Both receive RoundStart
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::RoundStart { round: 1, .. }));
    assert!(matches!(recv(&mut guest_ws).await, ServerMessage::RoundStart { round: 1, .. }));
}

#[tokio::test]
async fn join_nonexistent_game_returns_not_found() {
    let server = spawn_test_server().await;
    let mut ws = connect_ephemeral(&server).await;

    ws.send(join_game_msg("xyz999", "Bob")).await.unwrap();

    assert_eq!(recv(&mut ws).await, ServerMessage::GameNotFound);
}

#[tokio::test]
async fn duplicate_names_get_discriminator() {
    let server = spawn_test_server().await;

    // Host creates game as "Alice"
    let mut host_ws = connect_ephemeral(&server).await;
    host_ws.send(create_game_msg("Alice")).await.unwrap();

    let game_id = match recv(&mut host_ws).await {
        ServerMessage::GameCreated { game_id } => game_id,
        other => panic!("Expected GameCreated, got {:?}", other),
    };
    assert_eq!(recv(&mut host_ws).await, ServerMessage::WaitingForOpponent);

    // Guest joins with same name "Alice"
    let mut guest_ws = connect_ephemeral(&server).await;
    guest_ws.send(join_game_msg(&game_id, "Alice")).await.unwrap();

    // Host should see opponent as "Alice (2)"
    assert!(matches!(
        recv(&mut host_ws).await,
        ServerMessage::OpponentJoined { opponent_name } if opponent_name == "Alice (2)"
    ));
}

#[tokio::test]
async fn opponent_disconnect_notifies_remaining_player() {
    let server = spawn_test_server().await;

    // Host creates game
    let mut host_ws = connect_ephemeral(&server).await;
    host_ws.send(create_game_msg("Alice")).await.unwrap();

    let game_id = match recv(&mut host_ws).await {
        ServerMessage::GameCreated { game_id } => game_id,
        other => panic!("Expected GameCreated, got {:?}", other),
    };
    assert_eq!(recv(&mut host_ws).await, ServerMessage::WaitingForOpponent);

    // Guest joins
    let mut guest_ws = connect_ephemeral(&server).await;
    guest_ws.send(join_game_msg(&game_id, "Bob")).await.unwrap();

    // Skip to game started
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::OpponentJoined { .. }));
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::GameStart { .. }));
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::RoundStart { .. }));

    assert!(matches!(recv(&mut guest_ws).await, ServerMessage::GameStart { .. }));
    assert!(matches!(recv(&mut guest_ws).await, ServerMessage::RoundStart { .. }));

    // Guest disconnects
    guest_ws.close(None).await.unwrap();

    // Host should receive OpponentDisconnected
    let msg = recv(&mut host_ws).await;
    assert!(matches!(msg, ServerMessage::OpponentDisconnected));
}

#[tokio::test]
async fn rematch_starts_new_game_when_both_request() {
    let server = spawn_test_server().await;

    // Host creates game
    let mut host_ws = connect_ephemeral(&server).await;
    host_ws.send(create_game_msg("Alice")).await.unwrap();

    let game_id = match recv(&mut host_ws).await {
        ServerMessage::GameCreated { game_id } => game_id,
        other => panic!("Expected GameCreated, got {:?}", other),
    };
    assert_eq!(recv(&mut host_ws).await, ServerMessage::WaitingForOpponent);

    // Guest joins
    let mut guest_ws = connect_ephemeral(&server).await;
    guest_ws.send(join_game_msg(&game_id, "Bob")).await.unwrap();

    // Skip to game started
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::OpponentJoined { .. }));
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::GameStart { .. }));

    assert!(matches!(recv(&mut guest_ws).await, ServerMessage::GameStart { .. }));

    // Win 10 rounds to end the game (WINS_NEEDED=10)
    for _ in 0..10 {
        let ServerMessage::RoundStart { kanji, .. } = recv(&mut host_ws).await else {
            panic!("Expected RoundStart");
        };
        assert!(matches!(recv(&mut guest_ws).await, ServerMessage::RoundStart { .. }));

        host_ws.send(answer_msg(get_reading(&kanji))).await.unwrap();

        // Both receive RoundResult
        assert!(matches!(recv(&mut host_ws).await, ServerMessage::RoundResult { .. }));
        assert!(matches!(recv(&mut guest_ws).await, ServerMessage::RoundResult { .. }));
    }

    // Both receive GameEnd (host won)
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::GameEnd { winner: Some(_) }));
    assert!(matches!(recv(&mut guest_ws).await, ServerMessage::GameEnd { winner: Some(_) }));

    // Now both request rematch
    host_ws.send(rematch_msg()).await.unwrap();
    // First player should receive RematchWaiting
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::RematchWaiting));

    guest_ws.send(rematch_msg()).await.unwrap();

    // Both should receive GameStart (to reset frontend state)
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::GameStart { .. }));
    assert!(matches!(recv(&mut guest_ws).await, ServerMessage::GameStart { .. }));

    // Then RoundStart for the new game
    assert!(matches!(recv(&mut host_ws).await, ServerMessage::RoundStart { round: 1, .. }));
    assert!(matches!(recv(&mut guest_ws).await, ServerMessage::RoundStart { round: 1, .. }));
}
