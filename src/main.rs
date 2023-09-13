use std::{collections::HashMap, sync::Arc, io::Write};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufReader, BufWriter},
    net::{TcpListener, TcpStream},
    task::JoinSet,
};

#[tokio::main]
async fn main() {
    let listen = TcpListener::bind("192.168.1.11:3000").await.unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<TcpStream>();
    let task = tokio::spawn(async move {
        let mut vec = Vec::new();
        loop {
            //println!("execute");
            match rx.try_recv() {
                Ok(stream) => vec.push(stream),
                Err(e) => {
                    match e {
                        tokio::sync::mpsc::error::TryRecvError::Empty =>{

                        },
                        tokio::sync::mpsc::error::TryRecvError::Disconnected => {
                            return;
                        },
                    }
                }
            };
            for s in &vec {
                let mut buff = [0u8;1024];
                match s.try_read(&mut buff){
                    Ok(s)=>{
                        if s!=0{
                            println!("{s:?}\n {buff:?}");
                        }
                    }
                    Err(_)=>{}
                }
            }
        }
    });
    let mut join_set = JoinSet::new();
    while let Ok((stream, addr)) = listen.accept().await {
        tx.send(stream).unwrap();
        join_set.spawn(async move {
            // loop{
            //     let mut len_buf = [0u8;2];
            //     stream.readable().await;
            //     match stream.try_read(& mut len_buf){
            //         Ok(size)=>{
            //             if size == 2{
            //                 let len = u16::from_be_bytes(len_buf);
            //                 let mut buffer = Vec::new();
            //                 buffer.resize(len as usize, b'\0');
            //                 match stream.try_read(& mut buffer){
            //                     Ok(_)=>{
            //                         println!("read out buffer:\n{addr:?}\n{buffer:?}");
            //                         // if let Some(other_stream)  = find_other(& mut *map.lock().unwrap(),addr.to_string()){
            //                         //     other_stream.write(&buffer).await.unwrap();
            //                         // }

            //                     }
            //                     Err(_)=>{
            //                         println!("read buffer error");
            //                     }
            //                 }
            //             }else{
            //                 continue;
            //             }
            //         }
            //         Err(_)=>{}
            //       }
            // }
        });
    }
}


fn void_main(){
    let mut socket = std::net::TcpStream::connect("192.168.1.11").unwrap();
    let buff = b"123456789abcdefg";
    socket.write_all(buff).unwrap();
}
