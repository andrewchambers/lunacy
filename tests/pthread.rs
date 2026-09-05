use lunacy::{
    Errno,
    pthread::{self, JoinHandle},
};
use std::{
    cell::Cell,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

struct CountDrop(Arc<AtomicUsize>);
impl Drop for CountDrop {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn wait_finished<T>(handle: &JoinHandle<T>) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !handle.is_finished() {
        assert!(Instant::now() < deadline, "worker did not finish");
        pthread::yield_now().unwrap();
    }
}

#[test]
fn join_transfers_result_and_drops_captures_once() {
    let drops = Arc::new(AtomicUsize::new(0));
    let capture = CountDrop(Arc::clone(&drops));
    let text = String::from("from pthread");
    let handle = pthread::spawn(move || {
        drop(capture);
        text
    })
    .unwrap();
    assert!(!handle.is_current());
    assert_eq!(handle.join().unwrap(), "from pthread");
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn handle_can_move_threads_with_a_send_but_not_sync_result() {
    let handle = pthread::spawn(|| Cell::new(42)).unwrap();
    let value = std::thread::spawn(move || handle.join().unwrap())
        .join()
        .unwrap();
    assert_eq!(value.get(), 42);
}

#[test]
fn completion_flag_changes_only_after_closure_returns() {
    let (release, waiting) = mpsc::channel();
    let handle = pthread::spawn(move || {
        waiting.recv().unwrap();
        7
    })
    .unwrap();
    assert!(!handle.is_finished());
    release.send(()).unwrap();
    wait_finished(&handle);
    assert_eq!(handle.join(), Ok(7));
}

struct NotifyDrop(mpsc::Sender<()>);
impl Drop for NotifyDrop {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

#[test]
fn dropping_handle_detaches_and_reclaims_later_result() {
    let (release, waiting) = mpsc::channel();
    let (destroyed, notification) = mpsc::channel();
    let handle = pthread::spawn(move || {
        waiting.recv().unwrap();
        NotifyDrop(destroyed)
    })
    .unwrap();
    drop(handle);
    release.send(()).unwrap();
    notification.recv_timeout(Duration::from_secs(5)).unwrap();
}

#[test]
fn explicit_detach_reclaims_an_already_finished_result() {
    let (destroyed, notification) = mpsc::channel();
    let handle = pthread::spawn(move || NotifyDrop(destroyed)).unwrap();
    wait_finished(&handle);
    handle.detach().unwrap();
    notification.recv_timeout(Duration::from_secs(5)).unwrap();
}

#[test]
fn custom_stack_and_invalid_stack_cleanup() {
    assert_eq!(
        pthread::spawn_with_stack_size(1024 * 1024, || 19)
            .unwrap()
            .join(),
        Ok(19)
    );
    let drops = Arc::new(AtomicUsize::new(0));
    let capture = CountDrop(Arc::clone(&drops));
    let failed = pthread::spawn_with_stack_size(1, move || drop(capture));
    assert_eq!(failed.err(), Some(Errno::EINVAL));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn self_join_is_an_error_and_still_cleans_up() {
    let (send_handle, receive_handle) = mpsc::channel::<JoinHandle<()>>();
    let (done, notification) = mpsc::channel();
    let handle = pthread::spawn(move || {
        let own_handle = receive_handle.recv().unwrap();
        assert!(own_handle.is_current());
        let result = own_handle.join();
        done.send(result).unwrap();
    })
    .unwrap();
    assert!(send_handle.send(handle).is_ok());
    assert_eq!(
        notification.recv_timeout(Duration::from_secs(5)).unwrap(),
        Err(Errno::EDEADLK)
    );
}

#[test]
fn panic_child() {
    if std::env::var_os("LUNACY_PTHREAD_PANIC_CHILD").is_some() {
        let _ = pthread::spawn(|| panic!("lunacy pthread panic probe"))
            .unwrap()
            .join();
    }
}

#[test]
fn worker_panic_aborts_at_the_c_boundary() {
    use std::os::unix::process::ExitStatusExt;
    // Disable core dumps only in the child, then exec this one isolated test.
    let output = std::process::Command::new("sh")
        .args([
            "-c",
            "ulimit -c 0; exec \"$1\" --exact panic_child --nocapture",
            "lunacy-panic-test",
        ])
        .arg(std::env::current_exe().unwrap())
        .env("LUNACY_PTHREAD_PANIC_CHILD", "1")
        .output()
        .unwrap();
    assert!(
        output.status.signal().is_some(),
        "child did not abort: {:?}",
        output.status
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("lunacy pthread panic probe"));
}
