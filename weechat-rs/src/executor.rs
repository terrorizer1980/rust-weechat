pub use async_task::JoinHandle;
use pipe_channel::{channel, Receiver, Sender};
use std::collections::VecDeque;
use std::future::Future;
use std::sync::{Arc, Mutex, Weak};

use crate::hooks::{FdHook, FdHookMode, FdHookCallback};
use crate::Weechat;

static mut _EXECUTOR: Option<WeechatExecutor> = None;

type BufferName = String;

type Job = async_task::Task<()>;
type BufferJob = async_task::Task<BufferName>;

enum ExecutorJob {
    Job(Job),
    BufferJob(BufferJob),
}

type FutureQueue = Arc<Mutex<VecDeque<ExecutorJob>>>;
type FutureQueueHandle = Weak<Mutex<VecDeque<ExecutorJob>>>;

#[derive(Clone)]
pub struct WeechatExecutor {
    _hook: Arc<Mutex<Option<FdHook<Receiver<()>>>>>,
    sender: Arc<Mutex<Sender<()>>>,
    futures: FutureQueue,
}

impl FdHookCallback for WeechatExecutor {
    type FdObject = Receiver<()>;

    fn callback(&mut self, weechat: &Weechat, receiver: &mut Receiver<()>) {
        if receiver.recv().is_err() {
            return;
        }

        let mut futures = self.futures.lock().unwrap();
        let task = futures.pop_front();

        // Drop the lock here so we can spawn new futures from the currently
        // running one.
        drop(futures);

        if let Some(task) = task {
            match task {
                ExecutorJob::Job(t) => t.run(),
                ExecutorJob::BufferJob(t) => {
                    let weechat = unsafe { Weechat::weechat() };
                    let buffer_name = t.tag();

                    let buffer = weechat.buffer_search("==", buffer_name);

                    if buffer.is_some() {
                        t.run();
                    } else {
                        t.cancel();
                    }
                }
            }
        }
    }
}

impl WeechatExecutor {
    fn new() -> Self {
        let weechat = unsafe { Weechat::weechat() };

        let (sender, receiver) = channel();
        let sender = Arc::new(Mutex::new(sender));
        let queue = Arc::new(Mutex::new(VecDeque::new()));

        let mut executor = WeechatExecutor {
            _hook: Arc::new(Mutex::new(None)),
            sender,
            futures: queue,
        };

        let hook = weechat
            .hook_fd(
                receiver,
                FdHookMode::Read,
                executor.clone(),
            )
            .expect("Can't create executor FD hook");

        *executor._hook.lock().unwrap() = Some(hook);

        executor
    }

    pub fn free() {
        unsafe {
            _EXECUTOR.take();
        }
    }

    pub fn start() {
        let executor = WeechatExecutor::new();
        unsafe {
            _EXECUTOR = Some(executor);
        }
    }

    pub fn spawn<F, R>(future: F) -> JoinHandle<R, ()>
    where
        F: Future<Output = R> + 'static,
        R: 'static,
    {
        let executor =
            unsafe { _EXECUTOR.as_ref().expect("Executor wasn't started") };

        let sender = Arc::downgrade(&executor.sender);
        let queue = Arc::downgrade(&executor.futures);

        let schedule = move |task| {
            let sender = sender.upgrade();
            let queue = queue.upgrade();

            if let Some(q) = queue {
                let sender = sender
                    .expect("Futures queue exists but the channel got dropped");
                let mut weechat_notify = sender
                    .lock()
                    .expect("Weechat notification sender lock is poisoned");

                let mut queue = q.lock().expect(
                    "Lock of the future queue of the Weechat executor is poisoned",
                );

                queue.push_back(ExecutorJob::Job(task));
                weechat_notify
                    .send(())
                    .expect("Can't notify Weechat to run a future");
            }
        };

        let (task, handle) = async_task::spawn_local(future, schedule, ());

        task.schedule();

        handle
    }

    pub(crate) fn spawn_buffer_cb<F, R>(
        buffer_name: String,
        future: F,
    ) -> JoinHandle<R, String>
    where
        F: Future<Output = R> + 'static,
        R: 'static,
    {
        let executor =
            unsafe { _EXECUTOR.as_ref().expect("Executor wasn't started") };

        let sender = Arc::downgrade(&executor.sender);
        let queue = Arc::downgrade(&executor.futures);

        let schedule = move |task| {
            let sender = sender.upgrade();
            let queue = queue.upgrade();

            if let Some(q) = queue {
                let sender = sender
                    .expect("Futures queue exists but the channel got dropped");
                let mut weechat_notify = sender
                    .lock()
                    .expect("Weechat notification sender lock is poisoned");

                let mut queue = q.lock().expect(
                    "Lock of the future queue of the Weechat executor is poisoned",
                );

                queue.push_back(ExecutorJob::BufferJob(task));
                weechat_notify
                    .send(())
                    .expect("Can't notify Weechat to run a future");
            }
        };

        let (task, handle) =
            async_task::spawn_local(future, schedule, buffer_name);

        task.schedule();

        handle
    }
}
