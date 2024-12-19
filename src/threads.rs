use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Sender<Job>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        let (tx, rx) = mpsc::channel();
        let rx = Arc::new(Mutex::new(rx));
        let mut workers = Vec::new();
        for id in 1..=size {
            let shared_rx = Arc::clone(&rx);
            workers.push(Worker::new(id, shared_rx));
        }
        ThreadPool {
            workers,
            sender: tx,
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }

    pub fn join(self) {
        for worker in self.workers {
            worker.thread.join().unwrap();
        }
    }
}

struct Worker {
    #[warn(dead_code)]
    id: usize,
    thread: JoinHandle<Job>,
}

impl Worker {
    fn new(id: usize, rx: Arc<Mutex<Receiver<Job>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let job = rx.lock().unwrap().recv().unwrap();
            println!("Received a job with id {}", id);
            job();
        });

        Worker { id, thread }
    }
}
