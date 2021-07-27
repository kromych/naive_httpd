use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

#[derive(Debug)]
pub enum PoolError {
    InvalidPoolSize,
}

pub enum Message {
    NewJob(Job),
    ShutDown,
}

pub trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<Self>) {
        (*self)();
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    pub fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Self {
        Worker {
            thread: Some(thread::spawn(move || loop {
                let message = receiver.lock().unwrap().recv().unwrap();
                match message {
                    Message::NewJob(job) => {
                        log::info!("Worker {}: executing job...", id);

                        job.call_box();
                    }
                    Message::ShutDown => {
                        log::info!("Worker {}: shutting down!", id);

                        break;
                    }
                }
            })),
            id,
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for worker in &mut self.workers {
            log::info!("Requesting worker {} to shut down", worker.id);
            self.sender.send(Message::ShutDown).unwrap();
        }

        for worker in &mut self.workers {
            log::info!("Waiting for worker {} to shut down", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

pub type Job = Box<dyn FnBox + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Result<Self, PoolError> {
        if size == 0 {
            return Err(PoolError::InvalidPoolSize);
        }

        if size > 4 {
            return Err(PoolError::InvalidPoolSize);
        }

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        Ok(ThreadPool {
            workers: (0..size)
                .map(|i| Worker::new(i, Arc::clone(&receiver)))
                .collect(),
            sender,
        })
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(Message::NewJob(job)).unwrap();
    }
}
