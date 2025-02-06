use solver::{Evaluator, Mode, Solver, State};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Player {
    X,
    O,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Field {
    Empty,
    Player(Player),
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Board {
    fields: [[Field; 3]; 3],
    player: Player,
}

impl Board {
    pub fn won(&self) -> Option<Player> {
        let patterns = [
            [(0, 0), (0, 1), (0, 2)],
            [(1, 0), (1, 1), (1, 2)],
            [(2, 0), (2, 1), (2, 2)],
            [(0, 0), (1, 0), (2, 0)],
            [(0, 1), (1, 1), (2, 1)],
            [(0, 2), (1, 2), (2, 2)],
            [(0, 0), (1, 1), (2, 2)],
            [(0, 2), (1, 1), (2, 0)],
        ];
        for pattern in patterns.iter() {
            let mut player = None;
            for &(x, y) in pattern.iter() {
                match self.fields[x][y] {
                    Field::Empty => {
                        player = None;
                        break;
                    }
                    Field::Player(p) => {
                        if let Some(p2) = player {
                            if p != p2 {
                                player = None;
                                break;
                            }
                        } else {
                            player = Some(p);
                        }
                    }
                }
            }
            if let Some(p) = player {
                return Some(p);
            }
        }
        None
    }
}

pub struct Eval(Player);

impl Evaluator for Eval {
    type State = Board;
    type Value = f64;

    fn evaluate(&self, state: &Self::State) -> Self::Value {
        if let Some(player) = state.won() {
            if player == self.0 { 1.0 } else { -1.0 }
        } else {
            0.0
        }
    }

    fn mode(&self, state: &Self::State) -> Mode {
        if self.0 != state.player {
            Mode::Maximize
        } else {
            Mode::Minimize
        }
    }

    fn contemplate(&self, state: &Self::State, depth: usize) -> bool {
        true
    }
}

impl State for Board {
    type Change = (usize, usize);
    fn changes(&self) -> impl Iterator<Item = (f64, Self::Change)> {
        let won = if let Some(_) = self.won() {
            true
        } else {
            false
        };
        self.fields
            .iter()
            .enumerate()
            .map(|(i, f)| {
                f.iter().enumerate().filter_map(move |(j, &field)| {
                    if field == Field::Empty {
                        Some((1.0, (i, j)))
                    } else {
                        None
                    }
                })
            })
            .flatten()
            .filter(move |_| !won)
    }
    fn apply(&self, action: Self::Change) -> Self {
        let mut next = self.clone();
        next.fields[action.0][action.1] = Field::Player(self.player);
        next.player = match self.player {
            Player::X => Player::O,
            Player::O => Player::X,
        };
        next
    }
}

impl std::fmt::Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for row in self.fields.iter() {
            for field in row.iter() {
                match field {
                    Field::Empty => write!(f, " ")?,
                    Field::Player(Player::X) => write!(f, "X")?,
                    Field::Player(Player::O) => write!(f, "O")?,
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

fn main() {
    let board = Board {
        fields: [[Field::Empty; 3]; 3],
        player: Player::X,
    };
    let eval = Eval(Player::O);
    let mut solver = Solver::new(eval, board);
    loop {
        println!("{}", solver.state());
        if let Some(player) = solver.state().won() {
            println!(
                "{} won",
                match player {
                    Player::X => "X",
                    Player::O => "O",
                }
            );
            break;
        }
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let action = input.trim().split(' ').collect::<Vec<_>>();
        let action: (usize, usize) = (action[0].parse().unwrap(), action[1].parse().unwrap());
        solver.select(action);
        if let Some(player) = solver.state().won() {
            println!(
                "{} won",
                match player {
                    Player::X => "X",
                    Player::O => "O",
                }
            );
            break;
        }
        let optimal = solver.choose();
        if let Some((value, optimal)) = optimal {
            solver.select(optimal);
            dbg!(value);
        } else {
            println!("tie");
            break;
        }
    }
    dbg!(solver.state());
}
