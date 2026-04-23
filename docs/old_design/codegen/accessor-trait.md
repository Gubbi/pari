# accessor-trait

**Owning layer: `workspace`**

> **Superseded.** The accessor trait (`RoleAccessors`, `dyn`, `?Sized`, `Entity::Accessor`) was removed.
>
> Validators operate on tracked entities directly (`&TrackedRole`). Accessor methods are plain inherent async methods on each tracked struct — no trait, no trait object, no `#[async_trait]`.
>
> See [async-accessor-variants](async/async-accessor-variants.md) for current accessor and setter generation.
