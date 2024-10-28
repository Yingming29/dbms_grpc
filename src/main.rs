use dbms_grpc::dbms_service_server::{DbmsService, DbmsServiceServer};
use dbms_grpc::{ServerServerMsg, ClientServerMsg};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::net::SocketAddr;

use tokio::sync::mpsc;
use tonic::{transport::Server, Request, Response, Status};
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};


// use std::sync::Arc;


// Define the worker_id type
type WorkerId = u16;
// type WorkerSender = mpsc::Sender<>;

// Include the generated code from the .proto file
pub mod dbms_grpc {
    tonic::include_proto!("dbms_grpc");
}


// Define channels hold by a Worker Thread 
struct WorkerChannels<T> {
    /// The channel to receive high priority txn from the IO thread, ServerServer messages 
    txn_high_priority_rx: mpsc::Receiver<T>,    
    /// The channel to receive low priority txn from the IO thread, ClientServer messages
    txn_low_priority_rx: mpsc::Receiver<T>,
    /// The channel to send response for server to the IO thread
    server_response_tx: mpsc::Sender<T>,
    /// The channel to send response to for client the IO thread
    client_response_tx: mpsc::Sender<T>,
}

// Define channels hold by an IO Thread 
struct IOChannels<T> {
    /// The channel to send high priority txn to the Worker thread, ServerServer messages
    txn_high_priority_tx: mpsc::Sender<T>,
    /// The channel to send low priority txn to the Worker thread, ClientServer messages
    txn_low_priority_tx: mpsc::Sender<T>,
    /// The channel to receive response from the Worker thread for server
    server_response_rx: mpsc::Receiver<T>,
    /// The channel to receive response from the Worker thread for client
    client_response_rx: mpsc::Receiver<T>,
}


/// The service struct can store the shared data's reference
/// In this dbms, the data is transferred by Channels. 
/// TODO: add the shared arc of channels or data structure here. 
#[derive(Debug, Default)]
pub struct MyDbmsService {
    // Shared data structure accessible by both gRPC and worker threads
    shared_data: Arc<Mutex<HashMap<String, String>>>,
}

/// the DBMS service implementation 
#[tonic::async_trait]
impl DbmsService for MyDbmsService {       
    // rcp1: the client_server method bi-directional streaming. when the server receives a clientReq message. 
    type ClientServerStream = Pin<Box<dyn Stream<Item = Result<ClientServerMsg, Status>> + Send + Sync + 'static>>;

    async fn client_server(
        &self,
        request: Request<tonic::Streaming<ClientServerMsg>>,
    ) -> Result<Response<Self::ClientServerStream>, Status> {
        println!("ClientServerMsg stream started");

        let mut stream = request.into_inner();
        let shared_data = self.shared_data.clone();

        // Channel to send responses
        let (tx, rx) = mpsc::channel(100);
        // Spawn a task to handle incoming requests
        tokio::spawn(async move {
            while let Some(req) = stream.next().await {
                match req {
                    Ok(client_req) => {
                        println!("ClientServerService: Received ClientReq: {:?}", client_req);

                        // Access shared data
                        let mut data = shared_data.lock().unwrap();
                        data.insert(
                            format!("ClientServer Request: {}",client_req.id),
                            format!("Data: {:?}, Txn Type: {}", client_req.id, client_req.id),
                        );
                    }
                    Err(e) => {
                        eprintln!("Error receiving ClientReq: {:?}", e);
                        break;
                    }
                }
            }
            println!("ClientServerMsg stream ended");
        });

        // Return a stream of responses
        let response_stream = ReceiverStream::new(rx);

        Ok(Response::new(Box::pin(response_stream) as Self::ClientServerStream))
    }


    // rpc3: The second Bi-directional streaming RPC method for ServerServer communication 
    type ServerServerStream = Pin<Box<dyn Stream<Item = Result<ServerServerMsg, Status>> + Send + Sync + 'static>>;
    async fn server_server(
        &self,
        request: Request<tonic::Streaming<ServerServerMsg>>,
    ) -> Result<Response<Self::ServerServerStream>, Status> {
        println!("ServerServer stream started");

        let mut stream = request.into_inner();
        let shared_data = self.shared_data.clone();

        // Channel to send responses
        let (tx, rx) = mpsc::channel(100);

        // Spawn a task to handle incoming messages
        tokio::spawn(async move {
            while let Some(req) = stream.next().await {
                match req {
                    Ok(txn_msg) => {
                        println!("Received TxnMsg: {:?}", txn_msg);

                        // Access shared data
                        let mut data = shared_data.lock().unwrap();
                        data.insert(format!("txn_msg_{}", txn_msg.id), "Processed".to_string());

                        // Prepare a response
                        // let response = TxnMsg { name: txn_msg.name };

                        // // Send the response
                        // if tx.send(Ok(response)).await.is_err() {
                        //     eprintln!("Client disconnected");
                        //     break;
                        // }
                    }
                    Err(e) => {
                        eprintln!("Error receiving TxnMsg: {:?}", e);
                        break;
                    }
                }
            }
            println!("ServerServerMsg stream ended");
        });

        // Return a stream of responses
        let response_stream = ReceiverStream::new(rx);

        Ok(Response::new(Box::pin(response_stream) as Self::ServerServerStream))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("---- main.rs starts ----");

    // Define the address to listen on
    let addr: SocketAddr = "0.0.0.0:50051".parse()?;

    // Create an instance of your service
    let dbms_service = MyDbmsService::default();

    println!("DbmsService listening on {}", addr);

    // Start the gRPC server
    Server::builder()
        .add_service(DbmsServiceServer::new(dbms_service))
        .serve(addr)
        .await?;

    Ok(())
}
    


