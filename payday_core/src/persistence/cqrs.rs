use cqrs_es::persist::PersistedEventStore;
use cqrs_es::CqrsFramework;

pub type Cqrs<A, DB> = CqrsFramework<A, PersistedEventStore<DB, A>>;
