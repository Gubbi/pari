use super::{FixDomain, Recoverability, Severity};

pub trait ErrorCompose: sealed::AsAny + std::error::Error + Send + Sync + 'static {
    fn fix_domain(&self) -> FixDomain;
    fn recoverability(&self) -> Recoverability;
    fn severity(&self) -> Severity {
        Severity::from_classification(self.fix_domain(), self.recoverability())
    }
    /// For delegating enums: returns `&dyn Any` of the wrapped inner error.
    /// Default returns `None`; the `#[derive(ErrorCompose)]` macro overrides this for enums.
    fn as_any_inner(&self) -> Option<&dyn std::any::Any> {
        None
    }
}

impl dyn ErrorCompose {
    /// Downcast to a concrete error type.
    /// Checks the current node first, then the inner wrapped value for delegating enums.
    pub fn as_error<E: 'static>(&self) -> Option<&E> {
        if let Some(e) = self.as_any().downcast_ref::<E>() {
            return Some(e);
        }
        self.as_any_inner()?.downcast_ref::<E>()
    }
}

mod sealed {
    pub trait AsAny: 'static {
        fn as_any(&self) -> &dyn std::any::Any;
    }
    impl<T: 'static> AsAny for T {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
}
