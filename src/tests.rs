use crate::{Board, Meal, Player};
#[test]
fn test_test_optimal() {
    let almost_optimal_boards = [
        Board::new(vec![1, 0, 0, 0, 0], Player::RED),
        Board::new(vec![2, 1], Player::RED),
        Board::new(vec![2, 2, 3], Player::RED),
        Board::new(vec![2, 1, 1, 1], Player::RED),
        Board::new(vec![6, 1, 1, 5], Player::RED),
        Board::new(vec![7, 7, 4, 3, 2, 4], Player::RED),
    ];

    let other_almost_optimal_boards = [
        Board::new(vec![1, 0, 0, 0, 0], Player::RED),
        Board::new(vec![1, 3], Player::RED),
        Board::new(vec![1, 2, 6], Player::RED),
        Board::new(vec![1, 1, 1, 5], Player::RED),
    ];

    for board in &almost_optimal_boards {
        assert!(board.test_optimal(0, 1))
    }
    for board in almost_optimal_boards {
        assert!(!board.test_optimal(0, 0))
    }
    for (i, board) in other_almost_optimal_boards.iter().enumerate() {
        assert!(board.test_optimal(i, 1 + i as u8))
    }
}

#[test]
fn test_find_optimal() {
    let optimal_boards = [
        Board::new(vec![0, 0, 0, 0, 0], Player::RED),
        Board::new(vec![1, 1], Player::RED),
        Board::new(vec![1, 2, 3], Player::RED),
        Board::new(vec![1, 1, 1, 1], Player::RED),
        Board::new(vec![5, 1, 1, 5], Player::RED),
        Board::new(vec![6, 7, 4, 3, 2, 4], Player::RED),
    ];
    let non_optimal_boards = [
        Board::new(vec![0, 1, 0, 0, 0], Player::RED),
        Board::new(vec![1, 1, 1], Player::RED),
        Board::new(vec![1, 2, 4], Player::RED),
        Board::new(vec![5, 1, 2, 5], Player::RED),
        Board::new(vec![6, 7, 4, 3, 2, 5], Player::RED),
    ];
    let almost_optimal_boards = [
        Board::new(vec![1, 0, 0, 0, 0], Player::RED),
        Board::new(vec![2, 1], Player::RED),
        Board::new(vec![2, 2, 3], Player::RED),
        Board::new(vec![2, 1, 1, 1], Player::RED),
        Board::new(vec![6, 1, 1, 5], Player::RED),
        Board::new(vec![7, 7, 4, 3, 2, 4], Player::RED),
    ];

    let other_almost_optimal_boards = [
        Board::new(vec![1, 0, 0, 0, 0], Player::RED),
        Board::new(vec![1, 3], Player::RED),
        Board::new(vec![1, 2, 6], Player::RED),
        Board::new(vec![1, 1, 1, 5], Player::RED),
    ];

    for board in optimal_boards {
        assert!(board.find_optimal_move().is_none())
    }
    for board in non_optimal_boards {
        assert!(board.find_optimal_move().is_some())
    }
    for board in &almost_optimal_boards {
        assert_eq!(board.find_optimal_move(), Some(Meal { row_y: 0, amount: 1 }))
    }
    for (i, board) in other_almost_optimal_boards.iter().enumerate() {
        assert_eq!(board.find_optimal_move(), Some(Meal { row_y: i, amount: 1 + i as u8 }))
    }
}