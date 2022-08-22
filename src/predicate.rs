#[derive(Copy, Clone, Debug)]
pub(crate) enum PredicateStage {
    Initial,
    Changed,
}

pub(crate) trait Predicate<T>
where
    T: ?Sized,
{
    fn check(&mut self, data: &T, stage: PredicateStage) -> bool;
}

impl<F, T> Predicate<T> for F
where
    F: FnMut(&T) -> bool,
    T: ?Sized,
{
    fn check(&mut self, data: &T, _: PredicateStage) -> bool {
        self(data)
    }
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct ChangedPredicate;

impl<T> Predicate<T> for ChangedPredicate {
    fn check(&mut self, _: &T, stage: PredicateStage) -> bool {
        matches!(stage, PredicateStage::Changed)
    }
}
