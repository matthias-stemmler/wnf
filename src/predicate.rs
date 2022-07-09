#[derive(Debug, PartialEq)]
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

impl<T> Predicate<T> for PredicateStage {
    fn check(&mut self, _: &T, stage: PredicateStage) -> bool {
        *self == stage
    }
}
