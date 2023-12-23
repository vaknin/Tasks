#![allow(unused_imports)]
use tasks::task_service_client::TaskServiceClient;
use tasks::{ViewTasksRequest, CreateTaskRequest, UpdateTaskRequest};

pub mod tasks {
    tonic::include_proto!("taskmanager");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = TaskServiceClient::connect("http://127.0.0.1:50051").await?;
    
    let create_request = tonic::Request::new(CreateTaskRequest {
        description: "Walk the dog".to_string()
    });
    let create_response = client.create_task(create_request).await?;
    let created_id = create_response.into_inner().id;

    let update_request = tonic::Request::new(UpdateTaskRequest {
        id: created_id,
        completed: true
    });
    let update_response = client.update_task(update_request).await?;
    println!("Updated task: {:?}", update_response.into_inner());

    let view_request = tonic::Request::new(ViewTasksRequest {});
    let view_response = client.view_tasks(view_request).await?;
    println!("All Tasks: {:?}", view_response.into_inner().tasks);

    Ok(())
}
