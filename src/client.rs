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
    // 创建与 gRPC 服务器的连接
    let channel = Channel::from_static("http://127.0.0.1:50051").connect().await?;
    let mut client = DbmsServiceClient::new(channel);

    // 启动 client_server 双向流
    let (mut tx, rx) = tokio::sync::mpsc::channel(4);
    let response = client.client_server(tokio_stream::wrappers::ReceiverStream::new(rx)).await?;

    // 处理来自服务器的响应
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

    // 发送 ClientServerMsg 消息
    for i in 0..5 {
        let msg = ClientServerMsg { id: i };
        if tx.send(msg).await.is_err() {
            eprintln!("Server disconnected during client_server stream");
            break;
        }
    }
    drop(tx); // 显式关闭 client_server 的发送通道

    // 启动 server_server 双向流
    let (mut tx2, rx2) = tokio::sync::mpsc::channel(100);
    let response2 = client.server_server(tokio_stream::wrappers::ReceiverStream::new(rx2)).await?;

    // 处理来自服务器的响应
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

    // 发送 ServerServerMsg 消息
    for i in 0..5 {
        let msg = ServerServerMsg { id: i };
        if tx2.send(msg).await.is_err() {
            eprintln!("Server disconnected during server_server stream");
            break;
        }
    }
    drop(tx2); // 显式关闭 server_server 的发送通道

    // 添加延时，确保所有消息处理完毕后再关闭客户端程序
    sleep(Duration::from_secs(1)).await;

    Ok(())
}
