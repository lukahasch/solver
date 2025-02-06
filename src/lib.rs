#![feature(lazy_type_alias)]

use std::{
    collections::{BinaryHeap, HashMap},
    hash::Hash,
    ops::{Add, Div, Mul},
};

pub enum Mode {
    Minimize,
    Maximize,
}

pub trait State: Hash + PartialEq + Clone + Eq {
    type Change: Clone + Hash + PartialEq;
    fn apply(&self, action: Self::Change) -> Self;
    fn changes(&self) -> impl Iterator<Item = (f64, Self::Change)>;
}

pub trait Evaluator: Sized {
    type State: State;
    type Value: PartialOrd
        + Mul<f64, Output = Self::Value>
        + Div<f64, Output = Self::Value>
        + Clone
        + Default
        + std::fmt::Debug
        + Add<Self::Value, Output = Self::Value>;
    /// Evaluate a state not considering future states
    fn evaluate(&self, state: &Self::State) -> Self::Value;
    fn mode(&self, state: &Self::State) -> Mode;
    #[allow(unused)]
    fn contemplate(&self, state: &Self::State, depth: usize) -> bool {
        true
    }
}

pub type Cache<E: Evaluator> = HashMap<<E as Evaluator>::State, Possibility<E>>;

pub struct Solver<E: Evaluator> {
    evaluator: E,
    tree: Possibility<E>,
    cache: Cache<E>,
}

pub enum Possibility<E: Evaluator> {
    Leaf {
        state: E::State,
        value: E::Value,
    },
    Branch {
        state: E::State,
        children: Vec<(<E::State as State>::Change, f64, Possibility<E>)>,
    },
}

impl<E: Evaluator> Solver<E> {
    pub fn new(e: E, root: E::State) -> Self {
        let mut cache = HashMap::new();
        let tree = Possibility::new(root, &e, &mut cache);
        Solver {
            evaluator: e,
            tree,
            cache,
        }
    }

    pub fn choose(&mut self) -> Option<(E::Value, <E::State as State>::Change)> {
        self.tree.choose(&self.evaluator, &mut self.cache)
    }

    pub fn select(&mut self, change: <E::State as State>::Change) -> &mut Self {
        self.tree.select(change, &self.evaluator, &mut self.cache);
        self
    }

    pub fn state(&self) -> &E::State {
        self.tree.state()
    }
}

impl<E: Evaluator> Possibility<E> {
    pub fn new(root: E::State, e: &E, cache: &mut Cache<E>) -> Self {
        if let Some(possibility) = cache.get(&root) {
            return possibility.clone();
        }
        let value = e.evaluate(&root);
        Self::Leaf { state: root, value }
    }

    pub fn select(&mut self, change: <E::State as State>::Change, e: &E, cache: &mut Cache<E>) {
        self.expand(e, cache, 0);
        match self {
            Possibility::Leaf { .. } => {
                panic!("cannot select on leaf")
            }
            Possibility::Branch { children, .. } => {
                for (c, _, child) in children {
                    if *c == change {
                        *self = child.clone();
                        return;
                    }
                }
            }
        }
    }

    fn create(state: E::State, e: &E, cache: &mut Cache<E>, depth: usize) -> Self {
        if let Some(possibility) = cache.get(&state) {
            return possibility.clone();
        }
        let value = e.evaluate(&state);
        if !e.contemplate(&state, depth) {
            let value = e.evaluate(&state);
            return Self::Leaf { state, value };
        }
        let children: Vec<(<E::State as State>::Change, f64, Possibility<E>)> = state
            .changes()
            .map(|(weight, change)| {
                let child = Self::create(state.apply(change.clone()), e, cache, depth + 1);
                (change, weight, child)
            })
            .collect();
        let re = if children.is_empty() {
            Self::Leaf {
                state: state.clone(),
                value,
            }
        } else {
            Self::Branch {
                state: state.clone(),
                children,
            }
        };
        cache.insert(state, re.clone());
        re
    }

    pub fn expand(&mut self, e: &E, cache: &mut Cache<E>, depth: usize) {
        if !e.contemplate(self.state(), depth) {
            return;
        }
        match self {
            Possibility::Leaf { state, .. } => {
                let children: Vec<(<E::State as State>::Change, f64, Possibility<E>)> = state
                    .changes()
                    .map(|(weight, change)| {
                        let child = Self::create(state.apply(change.clone()), e, cache, depth + 1);
                        (change, weight, child)
                    })
                    .collect();
                if children.is_empty() {
                    return;
                }
                // possible optimization: don't clone state
                *self = Possibility::Branch {
                    state: state.clone(),
                    children,
                };
            }
            Possibility::Branch { children, .. } => {
                for (_, _, child) in children {
                    child.expand(e, cache, depth + 1);
                }
            }
        }
    }

    pub fn evaluate(&mut self, e: &E, cache: &mut Cache<E>, depth: usize) -> E::Value {
        // If the evaluator doesn’t want us to look ahead (or we are in a terminal state),
        // just return the immediate evaluation.
        if e.contemplate(self.state(), depth) {
            self.expand(e, cache, depth);
        }

        match self {
            Possibility::Leaf { value, .. } => value.clone(),
            Possibility::Branch { state, children } => {
                // Determine whether we are maximizing or minimizing at this state.
                let mode = e.mode(state);

                // Use a fold (or alternatively, iterate and track the best value)
                // to compute the best evaluation among the children.
                let child = children
                    .into_iter()
                    .map(|(_, weight, child)| child.evaluate(e, cache, depth + 1) * *weight)
                    .fold(None, |acc: Option<E::Value>, cur| match acc {
                        None => Some(cur),
                        Some(acc_val) => match mode {
                            Mode::Maximize => {
                                if cur > acc_val {
                                    Some(cur)
                                } else {
                                    Some(acc_val)
                                }
                            }
                            Mode::Minimize => {
                                if cur < acc_val {
                                    Some(cur)
                                } else {
                                    Some(acc_val)
                                }
                            }
                        },
                    })
                    .expect("There is at least one child.");

                // In this design we add the immediate evaluated value for the state
                // (for possible heuristic benefits) and then propagate the best child’s value.
                child
            }
        }
    }

    pub fn choose(
        &mut self,
        e: &E,
        cache: &mut Cache<E>,
    ) -> Option<(E::Value, <E::State as State>::Change)> {
        self.expand(e, cache, 0);
        match self {
            Possibility::Leaf { .. } => None,
            Possibility::Branch { children, .. } => {
                let mut heap = Vec::new();
                for (change, weight, child) in children {
                    let value = child.evaluate(e, cache, 1) * *weight;
                    heap.push((value, change));
                }
                heap.sort_by(|(a, _), (b, _)| {
                    b.partial_cmp(a).unwrap_or_else(|| {
                        dbg!(a, b);
                        unreachable!()
                    })
                });
                heap.pop().map(|(value, change)| (value, change.clone()))
            }
        }
    }

    pub fn state(&self) -> &E::State {
        match self {
            Possibility::Leaf { state, .. } => state,
            Possibility::Branch { state, .. } => state,
        }
    }
}

impl<E: Evaluator> Clone for Possibility<E> {
    fn clone(&self) -> Self {
        match self {
            Possibility::Leaf { state, value } => Possibility::Leaf {
                state: state.clone(),
                value: value.clone(),
            },
            Possibility::Branch { state, children } => Possibility::Branch {
                state: state.clone(),
                children: children.clone(),
            },
        }
    }
}
