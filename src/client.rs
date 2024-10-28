use dbms_grpc::dbms_service_client::DbmsServiceClient;
use dbms_grpc::{ClientServerMsg, ServerServerMsg};
use tonic::transport::Channel;
use tokio_stream::StreamExt;
use tokio::time::{sleep, Duration};

pub mod dbms_grpc {
    tonic::include_proto!("dbms_grpc");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // create channel to connect to server 
    let channel = Channel::from_static("http://127.0.0.1:50051").connect().await?;
    let mut client = DbmsServiceClient::new(channel);

    // init client_server bi-directional stream
    let (mut tx, rx) = tokio::sync::mpsc::channel(4);
    let response = client.client_server(tokio_stream::wrappers::ReceiverStream::new(rx)).await?;

    // process the response from server 
    let mut inbound = response.into_inner();
    tokio::spawn(async move {
        while let Some(msg) = inbound.next().await {
            match msg {
                Ok(response) => println!("Received response from server in client_server: {:?}", response),
                Err(e) => eprintln!("Error in client_server: {:?}", e),
            }
        }
        println!("client_server response stream ended.");
    });

    // send ClientServerMsg message
    for i in 0..5 {
        let msg = ClientServerMsg { id: i };
        if tx.send(msg).await.is_err() {
            eprintln!("Server disconnected during client_server stream");
            break;
        }
    }
    drop(tx); // close client_server send channel explicitly

    // init server_server bi-directional stream
    let (mut tx2, rx2) = tokio::sync::mpsc::channel(100);
    let response2 = client.server_server(tokio_stream::wrappers::ReceiverStream::new(rx2)).await?;

    // process the response from server
    let mut inbound2 = response2.into_inner();
    tokio::spawn(async move {
        while let Some(msg) = inbound2.next().await {
            match msg {
                Ok(response) => println!("Received response from server in server_server: {:?}", response),
                Err(e) => eprintln!("Error in server_server: {:?}", e),
            }
        }
        println!("server_server response stream ended.");
    });

    // send ServerServerMsg message
    for i in 0..5 {
        let msg = ServerServerMsg { id: i };
        if tx2.send(msg).await.is_err() {
            eprintln!("Server disconnected during server_server stream");
            break;
        }
    }
    drop(tx2); // close server_server send channel explicitly

    // wait for a while before exiting
    sleep(Duration::from_secs(1)).await;

    Ok(())
}




