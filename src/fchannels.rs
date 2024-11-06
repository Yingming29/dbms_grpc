/*
An simple example about the message communication between multiple async io-threads and multiple sync worker-threads. 
1. io threads send msg to a worker, who won the race in the flume channels's receivers. 
2. the workers send back response for this msg, which will also be received by only one io thread. 
*/


use flume::{Receiver, Sender};
use std::thread;
use tokio::runtime::Runtime;
use tonic::transport::Server;

// message from io to worker 
#[derive(Debug, Clone)]
struct Message {
    content: String,
}
// response from worker to io
#[derive(Debug, Clone)]
struct Response {
    content: String,
}

fn main() {
    // create two channels: from worker to io, and from io to worker
    let (io_to_worker_tx, io_to_worker_rx) = flume::unbounded::<Message>();
    let (worker_to_io_tx, worker_to_io_rx) = flume::unbounded::<Response>();

    // crate sync worker threads
    let num_workers = 4;
    let mut worker_handles = Vec::new();

    for i in 0..num_workers {
        let io_to_worker_rx = io_to_worker_rx.clone();
        let worker_to_io_tx = worker_to_io_tx.clone();
        let handle = thread::spawn(move || {
            worker_thread(i, io_to_worker_rx, worker_to_io_tx);
        });
        worker_handles.push(handle);
    }

    // tokio runtime
    let rt = Runtime::new().unwrap();

    rt.block_on(async {
        //  create multiple async io tasks
        let num_io_tasks = 4;
        let mut io_handles = Vec::new();

        for i in 0..num_io_tasks {
            let io_to_worker_tx = io_to_worker_tx.clone();
            let worker_to_io_rx = worker_to_io_rx.clone();
            let handle = tokio::spawn(async move {
                io_task(i, io_to_worker_tx, worker_to_io_rx).await;
            });
            io_handles.push(handle);
        }

        // wait for all io tasks to complete
        futures::future::join_all(io_handles).await;
    });
    // drop those tx and rx to close the channels, if NO one is using them
    drop(io_to_worker_tx);
    drop(worker_to_io_tx);
    // wait for all worker threads to complete
    for handle in worker_handles {
        handle.join().unwrap();
    }

    println!("complete");
}

// worker 
fn worker_thread(
    id: usize,
    io_to_worker_rx: Receiver<Message>,
    worker_to_io_tx: Sender<Response>,
) {
    while let Ok(msg) = io_to_worker_rx.recv() {
        println!("Worker {} receives：{:?}", id, msg);
        // process the message
        let response = Response {
            content: format!("Worker {} processes：{}", id, msg.content),
        };
        // send the response to io
        if worker_to_io_tx.send(response).is_err() {
            break; // channel is closed
        }
    }
    println!("Worker {} end.", id);
}

// io task
async fn io_task(
    id: usize,
    io_to_worker_tx: Sender<Message>,
    worker_to_io_rx: Receiver<Response>,
) {
    // send msg to worker 
    for i in 0..5 {
        let msg = Message {
            content: format!("IO {}'s message {}", id, i),
        };
        io_to_worker_tx.send_async(msg).await.unwrap();
    }

    // receive response from worker
    for _ in 0..5 {
        if let Ok(response) = worker_to_io_rx.recv_async().await {
            println!("IO {} receives response：{:?}", id, response);
        }
    }
}