use std::{collections::HashMap, sync::Arc};

// use rand::Rng;
use tokio::{
    io::AsyncReadExt,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpListener,
    },
    task::JoinHandle,
};

//use tun::TunPacket;

pub mod write_all;

//mod mock_reply;

#[allow(dead_code)]
enum Message {
    Add((u16, OwnedWriteHalf)),
    Data((u16, Vec<u8>)),
    Remove(u16),
    Mock((u16, Vec<u8>)),
}

async fn read_body(len: u16, reader: &mut OwnedReadHalf) -> Result<Vec<u8>, std::io::Error> {
    let len = len as usize;
    let mut buf = Vec::new();
    buf.resize(len as usize, b'\0');
    let mut read_len = 0;
    loop {
        match reader.read(&mut buf[read_len..]).await {
            Ok(size) => {
                if size == 0 {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::ConnectionAborted,
                        "",
                    ));
                }
                read_len += size;
                if read_len == len {
                    return Ok(buf);
                } else {
                    continue;
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

async fn find_another(
    map: &HashMap<u16, Arc<OwnedWriteHalf>>,
    me: u16,
) -> Option<&Arc<OwnedWriteHalf>> {
    for i in map {
        if *i.0 != me {
            return Some(i.1);
        }
    }
    return None;
}

enum AsyncMessage {
    Add((String, JoinHandle<()>)),
    Remove(String),
}

#[tokio::main]
async fn main() {
    let listen = TcpListener::bind("192.168.1.11:3000").await.unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Message>();
    let (tx_async, mut rx_async) = tokio::sync::mpsc::unbounded_channel::<AsyncMessage>();

    let establish_task = tokio::spawn(async move {
        let mut save_tasks = HashMap::new();
        loop {
            match rx_async.recv().await {
                Some(AsyncMessage::Add((id, task))) => {
                    save_tasks.insert(id, task);
                }
                Some(AsyncMessage::Remove(id)) => {
                    //println!("task complete, so removed");
                    save_tasks.remove(&id);
                }
                None => {}
            }
        }
    });
    let write_tasks = tokio::spawn(async move {
        let mut map = HashMap::new();
        loop {
            //println!("execute");
            match rx.recv().await {
                Some(Message::Add((num, writer))) => {
                    if map.len() >= 2 {
                        continue;
                    }
                    map.insert(num, Arc::new(writer));
                }
                Some(Message::Data((num, buff))) => {
                    //println!("receive data from {num}:\n{buff:?}");
                    match find_another(&map, num).await {
                        Some(writer) => {
                            let writer = Arc::clone(writer);
                            let uuid = uuid::Uuid::new_v4().to_string();
                            let tx_async_copy = tx_async.clone();
                            let _ = tx_async.send(AsyncMessage::Add((
                                uuid.clone(),
                                tokio::spawn(async move {
                                    write_all::write_all(writer, buff).await;
                                    let _ = tx_async_copy.send(AsyncMessage::Remove(uuid));
                                }),
                            )));
                        }
                        None => {}
                    }
                }
                Some(Message::Remove(num)) => {
                    //println!("remove {num}");
                    map.remove(&num);
                }
                Some(Message::Mock((_num, _buff))) => {
                    //println!("mock buff:\n {buff:?}");
                    // let writer = map.get(&num).unwrap().clone();
                    // let uuid = uuid::Uuid::new_v4().to_string();
                    // let tx_async_copy = tx_async.clone();
                    // let _ = tx_async.send(AsyncMessage::Add((
                    //     uuid.clone(),
                    //     tokio::spawn(async move {
                    //         let time = rand::thread_rng().gen_range(100..3000);
                    //         tokio::time::sleep(std::time::Duration::from_millis(time)).await;
                    //         mock_reply::parse_tun_packet(Some(Ok(TunPacket::new(buff))), writer)
                    //             .await;
                    //         let _ = tx_async_copy.send(AsyncMessage::Remove(uuid));
                    //     }),
                    // )));
                    //writer.write_all(src)
                }
                None => {
                    return;
                }
            };
        }
    });
    // let mut join_set = JoinSet::new();
    let mut index = 0;
    while let Ok((stream, _)) = listen.accept().await {
        let (mut reader, writer) = stream.into_split();
        let _ = tx.send(Message::Add((index, writer)));
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut len_buf = [0u8; 2];
            let mut read_header_len = 0;
            loop {
                match reader.read(&mut len_buf[read_header_len..]).await {
                    Ok(size) => {
                        //println!("read in comming {size}");
                        if size == 0 {
                            let _ = tx.send(Message::Remove(index));
                            return;
                        }
                        read_header_len += size;
                        if read_header_len == 2 {
                            read_header_len = 0;
                            let body_len = u16::from_be_bytes(len_buf);
                            //println!("body len {body_len}");
                            match read_body(body_len, &mut reader).await {
                                Ok(buf) => {
                                    //println!("ready body:\n {buf:?}");
                                    let _ = tx.send(Message::Data((index, buf)));
                                    //let _ = tx.send(Message::Mock((index, buf)));
                                }
                                Err(_) => {
                                    let _ = tx.send(Message::Remove(index));
                                    return;
                                }
                            }
                        } else {
                            continue;
                        }
                    }
                    Err(_) => {
                        let _ = tx.send(Message::Remove(index));
                        return;
                    }
                }
            }
        });
        index += 1;
    }
    write_tasks.await.unwrap();
    establish_task.await.unwrap();
}
