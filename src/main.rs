// pub mod dbms_grpc {
//     tonic::include_proto!("dbms_grpc");
// }
use dbms_grpc::dbms_service_server::{DbmsService, DbmsServiceServer};
use dbms_grpc::{ClientRep, ClientReq, TryConnectMsg, TxnMsg};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc;
use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};
use tonic::transport::Server;
use tonic::{Request, Response, Status};


// Include the generated code from the .proto file
pub mod dbms_grpc {
    tonic::include_proto!("dbms_grpc");
}

/// The service struct can store the shared data's reference
#[derive(Debug, Default)]
pub struct MyDbmsService {
    // Shared data structure accessible by both gRPC and worker threads
    shared_data: Arc<Mutex<HashMap<String, String>>>,
}

#[tonic::async_trait]
impl DbmsService for MyDbmsService {

    /// The unary RPC method 
    async fn try_connect(
        &self,
        request: Request<TryConnectMsg>,
    ) -> Result<Response<TryConnectMsg>, Status> {
        println!("unary method try_connect called");

        // Access the shared data, get the lock 
        let mut data = self.shared_data.lock().unwrap();
        data.insert("connection".to_string(), format!("Client {}", request.get_ref().name));


        // Echo back the message
        Ok(Response::new(request.into_inner()))
    }           
    //
    type ClientServerStream = Pin<Box<dyn Stream<Item = Result<ClientRep, Status>> + Send + Sync + 'static>>;


    // the client_server method bi-directional streaming. when the server receives a clientReq message. 
    async fn client_server(
        &self,
        request: Request<tonic::Streaming<ClientReq>>,
    ) -> Result<Response<Self::ClientServerStream>, Status> {
        println!("ClientServerMsg stream started");

        let mut stream = request.into_inner();
        let shared_data = self.shared_data.clone();

        // Channel to send responses
        let (tx, rx) = mpsc::channel(4);

        // Spawn a task to handle incoming requests
        tokio::spawn(async move {
            while let Some(req) = stream.next().await {
                match req {
                    Ok(client_req) => {
                        println!("Received ClientReq: {:?}", client_req);

                        // Access shared data
                        let mut data = shared_data.lock().unwrap();
                        data.insert(
                            format!("client_req_{}", client_req.name),
                            format!("Data: {:?}, Txn Type: {}", client_req.data, client_req.txn_type),
                        );

                        // Prepare a response
                        // for our client_server bi-directional streaming, the server will send back the txn result after the txn is done.
                        // Therefore, it will send back the clientRep message later. 
                        // let response = ClientRep { name: client_req.name };

                        // Send the response
                        // if tx.send(Ok(response)).await.is_err() {
                        //     eprintln!("Client disconnected");
                        //     break;
                        // }
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


    // The second Bi-directional streaming RPC method for ServerServer communication 
    type ServerServerStream = Pin<Box<dyn Stream<Item = Result<TxnMsg, Status>> + Send + Sync + 'static>>;
    async fn server_server(
        &self,
        request: Request<tonic::Streaming<TxnMsg>>,
    ) -> Result<Response<Self::ServerServerStream>, Status> {
        println!("ServerServer stream started");

        let mut stream = request.into_inner();
        let shared_data = self.shared_data.clone();

        // Channel to send responses
        let (tx, rx) = mpsc::channel(4);

        // Spawn a task to handle incoming messages
        tokio::spawn(async move {
            while let Some(req) = stream.next().await {
                match req {
                    Ok(txn_msg) => {
                        println!("Received TxnMsg: {:?}", txn_msg);

                        // Access shared data
                        let mut data = shared_data.lock().unwrap();
                        data.insert(format!("txn_msg_{}", txn_msg.name), "Processed".to_string());

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


fn main() {

    println!("---- main.rs starts ----");

}

// test the timeout of ssh

// fn main() {
//     // Initialize the shared data structure
//     let shared_data = Arc::new(SharedData::new());

//     // Start some worker threads
//     for i in 0..3 {
//         let shared_data_for_worker = shared_data.clone();
//         thread::spawn(move || {
//             println!("Worker {:?} started", current().id());
//             // Access or modify the shared data
//             let key = format!("worker_{}_key", i);
//             let value = format!("worker_{}_value", i);
//             shared_data_for_worker.insert(key, value);
//         });
//     }

//     // Start the gRPC server
//     let shared_data_for_service = shared_data.clone();

//     let rt = Runtime::new().unwrap();

//     // rt.block_on(async move {
//     //     let addr = "[::1]:50051".parse().unwrap();

//     //     let service = MyServiceImpl {
//     //         shared_data: shared_data_for_service,
//     //     };

//     //     println!("Starting gRPC server at {}", addr);

//     //     Server::builder()
//     //         .add_service(MyServiceServer::new(service))
//     //         .serve(addr)
//     //         .await
//     //         .unwrap();
//     // });
// }


