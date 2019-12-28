use crate::errors::Error;
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(PartialEq, Clone, Debug)]
enum Operation {
    Create {
        uuid: Uuid,
    },
    Update {
        uuid: Uuid,
        property: String,
        value: Value,
        timestamp: DateTime<Utc>,
    },
}

struct DB {
    // The current state, with all pending operations applied
    tasks: HashMap<Uuid, HashMap<String, Value>>,

    // The version at which `operations` begins
    base_version: u64,

    // Operations applied since `base_version`, in order.
    //
    // INVARIANT: Given a snapshot at `base_version`, applying these operations produces `tasks`.
    operations: Vec<Operation>,
}

impl DB {
    fn new() -> DB {
        DB {
            tasks: HashMap::new(),
            base_version: 0,
            operations: vec![],
        }
    }

    fn apply(&mut self, op: Operation) -> Result<(), Error> {
        match op {
            Operation::Create { uuid } => {
                match self.tasks.entry(uuid) {
                    Entry::Occupied(_) => {
                        return Err(Error::DBError(format!("Task {} already exists", uuid)));
                    }
                    ent @ Entry::Vacant(_) => {
                        ent.or_insert(HashMap::new());
                    }
                };
            }
            Operation::Update {
                ref uuid,
                ref property,
                ref value,
                timestamp: _,
            } => {
                match self.tasks.get_mut(uuid) {
                    Some(task) => {
                        task.insert(property.clone(), value.clone());
                    }
                    None => {
                        return Err(Error::DBError(format!("Task {} does not exist", uuid)));
                    }
                };
            }
        };
        self.operations.push(op);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_apply_create() {
        let mut db = DB::new();
        let uuid = Uuid::new_v4();
        let op = Operation::Create { uuid };
        db.apply(op.clone()).unwrap();

        let mut exp = HashMap::new();
        exp.insert(uuid, HashMap::new());
        assert_eq!(db.tasks, exp);
        assert_eq!(db.operations, vec![op]);
    }

    #[test]
    fn test_apply_create_exists() {
        let mut db = DB::new();
        let uuid = Uuid::new_v4();
        db.apply(Operation::Create { uuid }).unwrap();
        assert_eq!(
            db.apply(Operation::Create { uuid }),
            Err(Error::DBError(format!("Task {} already exists", uuid)))
        );
    }

    #[test]
    fn test_apply_create_update() {
        let mut db = DB::new();
        let uuid = Uuid::new_v4();
        let op1 = Operation::Create { uuid };
        db.apply(op1.clone()).unwrap();
        let op2 = Operation::Update {
            uuid,
            property: String::from("title"),
            value: Value::from("\"my task\""),
            timestamp: Utc::now(),
        };
        db.apply(op2.clone()).unwrap();

        let mut exp = HashMap::new();
        let mut task = HashMap::new();
        task.insert(String::from("title"), Value::from("\"my task\""));
        exp.insert(uuid, task);
        assert_eq!(db.tasks, exp);
        assert_eq!(db.operations, vec![op1, op2]);
    }

    #[test]
    fn test_apply_update_does_not_exist() {
        let mut db = DB::new();
        let uuid = Uuid::new_v4();
        assert_eq!(
            db.apply(Operation::Update {
                uuid,
                property: String::from("title"),
                value: Value::from("\"my task\""),
                timestamp: Utc::now(),
            }),
            Err(Error::DBError(format!("Task {} does not exist", uuid)))
        );
    }

}