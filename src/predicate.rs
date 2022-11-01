//! Types dealing with predicates on state data

#![deny(unsafe_code)]

/// A stage at which a predicate is evaluated
///
/// When evaluating a predicate on state data, the predicate can be evaluated both initially, i.e. before
/// subscribing to changes of the state data, or when the state data is changed.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum PredicateStage {
    /// The predicate is evaluated initially
    Initial,

    /// The predicate is evaluated on change
    Changed,
}

/// A predicate on state data
pub(crate) trait Predicate<T>
where
    T: ?Sized,
{
    /// Evaluates the predicate on the given data at the given stage
    fn check(&mut self, data: &T, stage: PredicateStage) -> bool;
}

/// Every `FnMut(&T) -> bool` closure is a predicate, where the stage of evaluation is irrelevant
impl<F, T> Predicate<T> for F
where
    F: FnMut(&T) -> bool,
    T: ?Sized,
{
    fn check(&mut self, data: &T, _: PredicateStage) -> bool {
        self(data)
    }
}

/// A special predicate that evaluates to `false` initially, but to `true` when the state data is changed,
/// regardless of the actual data
#[derive(Clone, Copy, Debug)]
pub(crate) struct ChangedPredicate;

impl<T> Predicate<T> for ChangedPredicate {
    fn check(&mut self, _: &T, stage: PredicateStage) -> bool {
        matches!(stage, PredicateStage::Changed)
    }
}
