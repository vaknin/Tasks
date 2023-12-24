#![allow(unused_imports)]

use std::collections::HashMap;
use std::sync::Mutex;

use redis::{Commands, RedisError};
use prost::Message;
use tokio::sync::oneshot;
use tonic::{transport::Server, Request, Response, Status};

use tasks::task_service_client::TaskServiceClient;
use tasks::task_service_server::{TaskService, TaskServiceServer};
use tasks::{ViewTasksRequest, CreateTaskRequest, UpdateTaskRequest, ViewTasksResponse, Task};

use errors::MyError;
mod errors;

pub mod tasks {
    tonic::include_proto!("taskmanager");
}

pub struct MyTaskService {
    pub redis_con: Mutex<redis::Connection>
}

impl MyTaskService {
    pub fn new(redis_con: redis::Connection) -> Self {
        MyTaskService { 
            redis_con: Mutex::new(redis_con) 
         }
    }
}

#[tonic::async_trait]
impl TaskService for MyTaskService {
    async fn view_tasks(&self, _request: Request<ViewTasksRequest>) -> Result<Response<ViewTasksResponse>, Status> {
        let mut redis_con = self.redis_con.lock().expect("can't lock redis con");
        let ids: Vec<i32> = redis_con.smembers("task_ids").map_err(MyError::from)?;
        let mut tasks = Vec::with_capacity(ids.len());
        for id in ids {
            let buf: Vec<u8> = redis_con.get(format!("task:{}", id)).map_err(MyError::from)?;
    
            let task = Task::decode(&buf[..]).map_err(MyError::from)?;
            tasks.push(task);
        }
        Ok(Response::new(ViewTasksResponse { tasks }))
    }

    async fn create_task(&self, request: Request<CreateTaskRequest>) -> Result<Response<Task>, Status> {
        // Create the new task
        let mut redis_con = self.redis_con.lock().expect("can't lock redis con");
        let id: i32 = redis_con.incr("task_ids_counter", 1).map_err(MyError::from)?;
        let new_task = Task {
            description: request.into_inner().description,
            completed: false,
            id
        };

        // Add to Redis
        let mut encoded_task: Vec<u8> = vec![];
        new_task.encode(&mut encoded_task).map_err(MyError::from)?;
        redis_con.set(format!("task:{}", id), encoded_task).map_err(MyError::from)?;
        redis_con.sadd("task_ids", id).map_err(MyError::from)?;
        Ok(Response::new(new_task))
    }

    async fn update_task(&self, request: Request<UpdateTaskRequest>) -> Result<Response<Task>, Status> {
        // Get the task
        let mut redis_con = self.redis_con.lock().expect("can't lock redis con");
        let request = request.into_inner();
        let encoded_task: Vec<u8> = redis_con.get(format!("task:{}", request.id)).map_err(MyError::from)?;
        let mut task = Task::decode(&encoded_task[..]).map_err(MyError::from)?;
        task.completed = request.completed;

        // Update the task
        let mut encoded_task: Vec<u8> = vec![];
        task.encode(&mut encoded_task).map_err(MyError::from)?;
        redis_con.set(format!("task:{}", request.id), encoded_task).map_err(MyError::from)?;
        Ok(Response::new(task))
    }
}

async fn start_server(tx: oneshot::Sender<bool>) -> Result<(), MyError>{
    println!("Starting the server task!");
    
    // Redis
    let client = redis::Client::open("redis://127.0.0.1/")?;
    let redis_con = client.get_connection()?;
    let task_service = MyTaskService::new(redis_con);
    
    // Server
    let addr = "127.0.0.1:50051".parse()?;
    let server = Server::builder()
        .add_service(TaskServiceServer::new(task_service));
    tx.send(true).unwrap();

    server.serve(addr).await?;
    Ok(())
}

async fn start_client(rx: oneshot::Receiver<bool>) -> Result<(), Box<dyn std::error::Error>> {
    rx.await?;
    println!("Starting the client task!");
    let mut client = TaskServiceClient::connect("http://127.0.0.1:50051").await?;
    let create_request = Request::new(CreateTaskRequest {
            description: "Walk the dog".to_string()
        });
    let create_response = client.create_task(create_request).await?;
    let created_id = create_response.into_inner().id;
    println!("{}", format!("created task with id: {}", created_id));

    // let update_request = Request::new(UpdateTaskRequest {
    //         id: created_id,
    //         completed: true
    // });
    // let update_response = client.update_task(update_request).await?;
    // println!("Updated task: {:?}", update_response.into_inner());
    //
    // let view_request = Request::new(ViewTasksRequest {});
    // let view_response = client.view_tasks(view_request).await?;
    // println!("All Tasks: {:?}", view_response.into_inner().tasks);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), MyError> {
    let (tx, rx) = oneshot::channel::<bool>();
    let server_task = tokio::spawn(async move {
        if let Err(e) = start_server(tx).await {
            eprintln!("Failed to start server: {}", e)
        }
    });

    let client_task = tokio::spawn(async move {
        match start_client(rx).await {
            Ok(_) => println!("Client started successfully"),
            Err(e) => eprintln!("Failed to start Client: {}", e)
        }
    });

    let _ = tokio::join!(server_task, client_task);

    Ok(())
}
