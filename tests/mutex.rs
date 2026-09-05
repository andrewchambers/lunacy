use lunacy::{Errno, pthread, sync::Mutex};
use std::{
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[test]
fn pthread_workers_serialize_access_to_non_sync_data() {
    let shared = Arc::new(Mutex::new(Cell::new(0usize)).unwrap());
    let mut threads = Vec::new();
    for _ in 0..6 {
        let shared = Arc::clone(&shared);
        threads.push(
            pthread::spawn(move || {
                for _ in 0..2000 {
                    let guard = shared.lock().unwrap();
                    guard.set(guard.get() + 1);
                }
            })
            .unwrap(),
        );
    }
    for thread in threads {
        thread.join().unwrap();
    }
    assert_eq!(shared.lock().unwrap().get(), 12000);
}

#[test]
fn recursive_lock_and_try_lock_report_native_errors() {
    let mutex = Mutex::new(5).unwrap();
    let mut guard = mutex.lock().unwrap();
    // Seed an unrelated errno: pthread return codes must be translated directly.
    let _ = std::fs::File::open("");
    assert_eq!(mutex.try_lock().err(), Some(Errno::EBUSY));
    assert_eq!(mutex.lock().err(), Some(Errno::EDEADLK));
    *guard = 9;
    drop(guard);
    assert_eq!(*mutex.try_lock().unwrap(), 9);
}

#[test]
fn try_lock_reports_contention_on_another_thread() {
    let mutex = Arc::new(Mutex::new(0).unwrap());
    let guard = mutex.lock().unwrap();
    let other = Arc::clone(&mutex);
    let worker = pthread::spawn(move || other.try_lock().err()).unwrap();
    assert_eq!(worker.join(), Ok(Some(Errno::EBUSY)));
    drop(guard);
    assert!(mutex.try_lock().is_ok());
}

#[test]
fn mutex_can_move_after_native_initialization() {
    let mut mutex = Mutex::new(String::from("a")).unwrap();
    mutex.get_mut().push('b');
    {
        mutex.lock().unwrap().push('c');
    }
    let boxed = Box::new(mutex);
    boxed.lock().unwrap().push('d');
    let moved = *boxed;
    assert_eq!(moved.into_inner(), "abcd");
}

struct CountDrop(Arc<AtomicUsize>);
impl Drop for CountDrop {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[test]
fn into_inner_and_forgotten_guards_do_not_double_drop_data() {
    let drops = Arc::new(AtomicUsize::new(0));
    let mutex = Mutex::new(CountDrop(Arc::clone(&drops))).unwrap();
    let data = mutex.into_inner();
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(data);
    assert_eq!(drops.load(Ordering::SeqCst), 1);

    let mutex = Mutex::new(CountDrop(Arc::clone(&drops))).unwrap();
    std::mem::forget(mutex.lock().unwrap());
    drop(mutex); // Must leak the locked native mutex, not destroy it (POSIX UB).
    assert_eq!(drops.load(Ordering::SeqCst), 2);

    let mutex = Mutex::new(CountDrop(Arc::clone(&drops))).unwrap();
    std::mem::forget(mutex.lock().unwrap());
    drop(mutex.into_inner());
    assert_eq!(drops.load(Ordering::SeqCst), 3);
}

#[test]
fn rust_unwinding_releases_guard_without_poisoning() {
    let mutex = Mutex::new(1).unwrap();
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut guard = mutex.lock().unwrap();
        *guard = 2;
        panic!("caught outside a pthread callback");
    }));
    assert!(result.is_err());
    assert_eq!(*mutex.lock().unwrap(), 2);
}
