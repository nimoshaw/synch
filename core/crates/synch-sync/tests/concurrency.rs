use std::sync::{Arc, Mutex};
use std::thread;
use synch_sync::{DeltaEntry, Vault};

#[test]
fn stress_test_concurrent_vault_access() {
    let vault = Arc::new(Mutex::new(Vault::new("stress-test-vault")));
    let mut handles = vec![];

    for i in 0..10 {
        let v = Arc::clone(&vault);
        let handle = thread::spawn(move || {
            for j in 0..100 {
                let node_id = format!("node-{}", i);
                let path = format!("file-{}.txt", j);
                let entry = DeltaEntry::new_create(
                    &path,
                    format!("content-{}", j).into_bytes(),
                    &node_id,
                    (j + 1) as u64,
                    1000 + (i * 100 + j) as u64,
                );
                let mut v_lock = v.lock().unwrap();
                v_lock.apply_delta(entry).unwrap();
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    let v_lock = vault.lock().unwrap();
    assert_eq!(v_lock.version, 1000);
}
