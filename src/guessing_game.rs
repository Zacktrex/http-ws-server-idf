//! Guessing game logic and state management

use core::cmp::Ordering;
use log::*;

/// Represents a single guessing game session
pub struct GuessingGame {
    guesses: u32,
    secret: u32,
    done: bool,
}

impl GuessingGame {
    /// Create a new guessing game with a secret number
    pub fn new(secret: u32) -> Self {
        info!("Creating new guessing game with secret: {}", secret);
        Self {
            guesses: 0,
            secret,
            done: false,
        }
    }

    /// Make a guess and return the comparison result and guess count
    pub fn guess(&mut self, guess: u32) -> (Ordering, u32) {
        if self.done {
            warn!("Attempted guess on completed game");
            (Ordering::Equal, self.guesses)
        } else {
            self.guesses += 1;
            let cmp = guess.cmp(&self.secret);
            info!(
                "Guess #{}: {} (secret: {}, result: {:?})",
                self.guesses, guess, self.secret, cmp
            );
            if cmp == Ordering::Equal {
                self.done = true;
                info!("Game completed in {} guesses", self.guesses);
            }
            (cmp, self.guesses)
        }
    }

    /// Parse a guess string into a valid number (1-100)
    pub fn parse_guess(input: &str) -> Option<u32> {
        // Trim control codes (including null bytes) and/or whitespace
        let Ok(number) = input
            .trim_matches(|c: char| c.is_ascii_control() || c.is_whitespace())
            .parse::<u32>()
        else {
            warn!("Not a number: `{input}` (length {})", input.len());
            return None;
        };

        if !(1..=100).contains(&number) {
            warn!("Not in range ({number})");
            return None;
        }

        info!("Parsed guess: {}", number);
        Some(number)
    }

    /// Get the secret number (for display after winning)
    pub fn secret(&self) -> u32 {
        self.secret
    }
}

#[cfg(test)]
mod tests {


    #[test]
    fn test_guessing_game_new() {
        let game = GuessingGame::new(42);
        assert_eq!(game.guesses, 0);
        assert_eq!(game.secret, 42);
    }

    #[test]
    fn test_guessing_game_guess_too_high() {
        let mut game = GuessingGame::new(50);
        let (cmp, count) = game.guess(75);
        assert_eq!(cmp, Ordering::Greater);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_guessing_game_guess_too_low() {
        let mut game = GuessingGame::new(50);
        let (cmp, count) = game.guess(25);
        assert_eq!(cmp, Ordering::Less);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_guessing_game_guess_correct() {
        let mut game = GuessingGame::new(50);
        let (cmp, count) = game.guess(50);
        assert_eq!(cmp, Ordering::Equal);
        assert_eq!(count, 1);
    }

    #[test]
    fn test_parse_guess_valid() {
        assert_eq!(GuessingGame::parse_guess("42"), Some(42));
        assert_eq!(GuessingGame::parse_guess("1"), Some(1));
        assert_eq!(GuessingGame::parse_guess("100"), Some(100));
    }

    #[test]
    fn test_parse_guess_invalid() {
        assert_eq!(GuessingGame::parse_guess("abc"), None);
        assert_eq!(GuessingGame::parse_guess("0"), None);
        assert_eq!(GuessingGame::parse_guess("101"), None);
    }
}

